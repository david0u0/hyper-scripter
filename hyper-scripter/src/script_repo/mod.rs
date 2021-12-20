use crate::error::Result;
use crate::script::{IntoScriptName, ScriptInfo, ScriptName};
use crate::tag::{Tag, TagFilterGroup};
use crate::Either;
use chrono::{Duration, Utc};
use fxhash::FxHashMap as HashMap;
use hyper_scripter_historian::{Event, EventData, Historian};
use sqlx::SqlitePool;
use std::collections::hash_map::Entry::{self, *};

pub mod helper;
pub use helper::RepoEntry;
use helper::*;

#[derive(Clone, Debug)]
pub struct RecentFilter {
    pub recent: u32,
    pub archaeology: bool,
}

#[derive(Debug)]
enum TraceOption {
    Normal,
    // record nothing
    NoTrace,
    // don't affect last time, only record history
    Humble,
}

#[derive(Debug)]
pub struct DBEnv {
    pub info_pool: SqlitePool,
    pub historian: Historian,
    trace_opt: TraceOption,
    modifies_script: bool,
}

pub struct RepoEntryOptional<'b> {
    entry: Entry<'b, String, ScriptInfo>,
    env: &'b DBEnv,
}
impl<'b> RepoEntryOptional<'b> {
    pub fn into_either(self) -> Either<RepoEntry<'b>, Self> {
        match self.entry {
            Occupied(entry) => Either::One(RepoEntry::new(entry.into_mut(), self.env)),
            _ => Either::Two(self),
        }
    }
    pub async fn or_insert(self, info: ScriptInfo) -> Result<RepoEntry<'b>> {
        let exist = matches!(&self.entry, Occupied(_));
        let info = self.entry.or_insert(info);
        if !exist {
            log::debug!("往資料庫塞新腳本 {:?}", info);
            let id = self.env.handle_insert(info).await?;
            log::debug!("往資料庫新增腳本成功，得 id = {}", id);
            info.set_id(id as i64);
        }
        Ok(RepoEntry::new(info, self.env))
    }
}

impl DBEnv {
    pub async fn handle_neglect(&self, id: i64) -> Result {
        let time = Utc::now().naive_utc();
        sqlx::query!(
            "UPDATE last_events SET neglect = ? WHERE script_id = ?",
            time,
            id
        )
        .execute(&self.info_pool)
        .await?;
        Ok(())
    }

    async fn update_last_time(&self, info: &ScriptInfo) -> Result {
        match self.trace_opt {
            TraceOption::NoTrace => return Ok(()),
            TraceOption::Normal => (),
            TraceOption::Humble => {
                let humble_time = Utc::now().naive_utc();
                sqlx::query!(
                    "
                    INSERT OR REPLACE INTO last_events
                    (script_id, humble)
                    VALUES(?, ?)
                    ",
                    info.id,
                    humble_time
                )
                .execute(&self.info_pool)
                .await?;
                return Ok(());
            }
        }

        let last_time = info.last_time();
        let exec_time = info.exec_time.as_ref().map(|t| **t);
        let exec_done_time = info.exec_done_time.as_ref().map(|t| **t);
        let neglect_time = info.neglect_time.as_ref().map(|t| **t);
        let miss_time = info.miss_time.as_ref().map(|t| **t);
        let exec_count = info.exec_count as i32;
        sqlx::query!(
            "
            INSERT OR REPLACE INTO last_events
            (script_id, last_time, read, write, miss, exec, exec_done, neglect, humble, exec_count)
            VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
            info.id,
            last_time,
            *info.read_time,
            *info.write_time,
            miss_time,
            exec_time,
            exec_done_time,
            neglect_time,
            info.humble_time,
            exec_count
        )
        .execute(&self.info_pool)
        .await?;
        Ok(())
    }

    async fn handle_delete(&self, id: i64) -> Result {
        assert!(self.modifies_script);
        self.historian.remove(id).await?;
        log::debug!("清理腳本 {:?} 的最新事件", id);
        sqlx::query!("DELETE FROM last_events WHERE script_id = ?", id)
            .execute(&self.info_pool)
            .await?;
        sqlx::query!("DELETE from script_infos where id = ?", id)
            .execute(&self.info_pool)
            .await?;
        Ok(())
    }

    async fn handle_insert(&self, info: &ScriptInfo) -> Result<i64> {
        assert!(self.modifies_script);
        let name_cow = info.name.key();
        let name = name_cow.as_ref();
        let ty = info.ty.as_ref();
        let tags = join_tags(info.tags.iter());
        sqlx::query!(
            "
            INSERT INTO script_infos (name, ty, tags)
            VALUES(?, ?, ?)
            ",
            name,
            ty,
            tags,
        )
        .execute(&self.info_pool)
        .await?;
        let id = sqlx::query!("SELECT last_insert_rowid() as id")
            .fetch_one(&self.info_pool)
            .await?
            .id;
        Ok(id as i64)
    }

    async fn handle_change(&self, info: &ScriptInfo) -> Result<i64> {
        log::debug!("開始修改資料庫 {:?}", info);
        if info.changed {
            assert!(self.modifies_script);
            let name = info.name.key();
            let name = name.as_ref();
            let tags = join_tags(info.tags.iter());
            let ty = info.ty.as_ref();
            sqlx::query!(
                "UPDATE script_infos SET name = ?, tags = ?, ty = ? where id = ?",
                name,
                tags,
                ty,
                info.id,
            )
            .execute(&self.info_pool)
            .await?;
        }

        if matches!(self.trace_opt, TraceOption::NoTrace) {
            return Ok(0);
        }

        let mut last_event_id = 0;
        macro_rules! record_event {
            ($time:expr, $data:expr) => {
                self.historian.record(&Event {
                    script_id: info.id,
                    humble: matches!(self.trace_opt, TraceOption::Humble),
                    time: $time,
                    data: $data,
                })
            };
        }

        if let Some(time) = info.exec_done_time.as_ref() {
            if let Some(&(code, main_event_id)) = time.data() {
                log::debug!("{:?} 的執行完畢事件", info.name);
                last_event_id = record_event!(
                    **time,
                    EventData::ExecDone {
                        code,
                        main_event_id,
                    }
                )
                .await?;

                if last_event_id != 0 {
                    self.update_last_time(info).await?;
                } else {
                    log::info!("{:?} 的執行完畢事件被忽略了", info.name);
                }
                return Ok(last_event_id); // XXX: 超級醜的作法，為了避免重復記錄其它的事件
            }
        }

        self.update_last_time(info).await?;

        if info.read_time.has_changed() {
            log::debug!("{:?} 的讀取事件", info.name);
            last_event_id = record_event!(*info.read_time, EventData::Read).await?;
        }
        if info.write_time.has_changed() {
            log::debug!("{:?} 的寫入事件", info.name);
            last_event_id = record_event!(*info.write_time, EventData::Write).await?;
        }
        if let Some(time) = info.miss_time.as_ref() {
            if time.has_changed() {
                log::debug!("{:?} 的錯過事件", info.name);
                last_event_id = record_event!(**time, EventData::Miss).await?;
            }
        }
        if let Some(time) = info.exec_time.as_ref() {
            if let Some((content, args, dir)) = time.data() {
                log::debug!("{:?} 的執行事件", info.name);
                last_event_id = record_event!(
                    **time,
                    EventData::Exec {
                        content,
                        args,
                        dir: dir.as_deref(),
                    }
                )
                .await?;
            }
        }

        Ok(last_event_id)
    }
}

fn join_tags<'a, I: Iterator<Item = &'a Tag>>(tags: I) -> String {
    let tags_arr: Vec<&str> = tags.map(|t| t.as_ref()).collect();
    tags_arr.join(",")
}

#[derive(Debug)]
pub struct ScriptRepo {
    map: HashMap<String, ScriptInfo>,
    hidden_map: HashMap<String, ScriptInfo>,
    latest_name: Option<String>,
    db_env: DBEnv,
}

impl ScriptRepo {
    pub fn iter(&self) -> impl Iterator<Item = &ScriptInfo> {
        self.map.iter().map(|(_, info)| info)
    }
    pub fn iter_mut(&mut self, all: bool) -> Iter<'_> {
        Iter {
            iter: self.map.iter_mut(),
            env: &self.db_env,
            iter2: if all {
                Some(self.hidden_map.iter_mut())
            } else {
                None
            },
        }
    }
    pub fn iter_hidden_mut(&mut self) -> Iter<'_> {
        Iter {
            iter: self.hidden_map.iter_mut(),
            iter2: None,
            env: &self.db_env,
        }
    }
    pub fn historian(&self) -> &Historian {
        &self.db_env.historian
    }
    pub async fn new(
        pool: SqlitePool,
        recent: Option<RecentFilter>,
        historian: Historian,
        modifies_script: bool,
    ) -> Result<ScriptRepo> {
        let mut hidden_map = HashMap::<String, ScriptInfo>::default();
        let time_bound = recent.map(|r| {
            let mut time = Utc::now().naive_utc();
            time -= Duration::days(r.recent.into());
            (time, r.archaeology)
        });

        let scripts = sqlx::query!(
            "SELECT * FROM script_infos si LEFT JOIN last_events le ON si.id = le.script_id"
        )
        .fetch_all(&pool)
        .await?;
        let mut map: HashMap<String, ScriptInfo> = Default::default();
        for record in scripts.into_iter() {
            let name = record.name;
            log::trace!("載入腳本：{} {} {}", name, record.ty, record.tags);
            let script_name = name.clone().into_script_name()?;

            let mut builder = ScriptInfo::builder(
                record.id,
                script_name,
                record.ty.into(),
                record.tags.split(',').filter_map(|s| {
                    if s.is_empty() {
                        None
                    } else {
                        // TODO: 錯誤處理，至少印個警告
                        s.parse().ok()
                    }
                }),
            );

            builder.created_time(record.created_time);
            builder.exec_count(record.exec_count as u64);
            if let Some(time) = record.write {
                builder.write_time(time);
            }
            if let Some(time) = record.read {
                builder.read_time(time);
            }
            if let Some(time) = record.miss {
                builder.miss_time(time);
            }
            if let Some(time) = record.exec {
                builder.exec_time(time);
            }
            if let Some(time) = record.exec_done {
                builder.exec_done_time(time);
            }
            if let Some(time) = record.neglect {
                builder.neglect_time(time);
            }
            if let Some(time) = record.humble {
                builder.humble_time(time);
            }

            let script = builder.build();

            let hide_by_time = if let Some((mut time_bound, archaeology)) = time_bound {
                if let Some(neglect) = record.neglect {
                    log::debug!("腳本 {} 曾於 {} 被忽略", script.name, neglect);
                    time_bound = std::cmp::max(neglect, time_bound);
                }
                let overtime = time_bound > script.last_major_time();
                archaeology ^ overtime
            } else {
                false
            };
            if hide_by_time {
                hidden_map.insert(name, script);
            } else {
                map.insert(name, script);
            }
        }
        Ok(ScriptRepo {
            map,
            hidden_map,
            latest_name: None,
            db_env: DBEnv {
                trace_opt: TraceOption::Normal,
                info_pool: pool,
                historian,
                modifies_script,
            },
        })
    }
    pub fn no_trace(&mut self) {
        self.db_env.trace_opt = TraceOption::NoTrace;
    }
    pub fn humble(&mut self) {
        self.db_env.trace_opt = TraceOption::Humble;
    }
    // fn latest_mut_no_cache(&mut self) -> Option<&mut ScriptInfo<'a>> {
    //     let latest = self.map.iter_mut().max_by_key(|(_, info)| info.last_time());
    //     if let Some((name, info)) = latest {
    //         self.latest_name = Some(name.clone());
    //         Some(info)
    //     } else {
    //         None
    //     }
    // }
    pub fn latest_mut(&mut self, n: usize, all: bool) -> Option<RepoEntry<'_>> {
        // if let Some(name) = &self.latest_name {
        //     // FIXME: 一旦 rust nll 進化就修掉這段
        //     if self.map.contains_key(name) {
        //         return self.map.get_mut(name);
        //     }
        //     log::warn!("快取住的最新資訊已經不見了…？重找一次");
        // }
        // self.latest_mut_no_cache()
        let mut v: Vec<_> = if all {
            self.map
                .iter_mut()
                .chain(self.hidden_map.iter_mut())
                .map(|(_, s)| s)
                .collect()
        } else {
            self.map.iter_mut().map(|(_, s)| s).collect()
        };
        v.sort_by_key(|s| s.last_time());
        if v.len() >= n {
            // SAFETY: 從向量中讀一個可變指針安啦
            let t = unsafe { std::ptr::read(&v[v.len() - n]) };
            Some(RepoEntry::new(t, &self.db_env))
        } else {
            None
        }
    }
    pub fn get_mut(&mut self, name: &ScriptName, all: bool) -> Option<RepoEntry<'_>> {
        // FIXME: 一旦 NLL 進化就修掉這個 unsafe
        let map = &mut self.map as *mut HashMap<String, ScriptInfo>;
        let map = unsafe { &mut *map };
        match (all, map.get_mut(&*name.key())) {
            (false, None) => None,
            (true, None) => self.get_hidden_mut(name),
            (_, Some(info)) => Some(RepoEntry::new(info, &self.db_env)),
        }
    }
    pub fn get_hidden_mut(&mut self, name: &ScriptName) -> Option<RepoEntry<'_>> {
        let db_env = &self.db_env;
        self.hidden_map
            .get_mut(&*name.key())
            .map(|info| RepoEntry::new(info, db_env))
    }
    pub fn get_mut_by_id(&mut self, id: i64) -> Option<RepoEntry<'_>> {
        // XXX: 複雜度很瞎
        self.iter_mut(true).find(|e| e.id == id)
    }

    pub async fn remove(&mut self, id: i64) -> Result {
        // TODO: 從 map 中刪掉？但如果之後沒其它用途似乎也未必需要...
        log::debug!("從資料庫刪除腳本 {:?}", id);
        self.db_env.handle_delete(id).await?;
        Ok(())
    }
    pub fn entry(&mut self, name: &ScriptName) -> RepoEntryOptional<'_> {
        // TODO: 決定要插 hidden 與否
        let entry = self.map.entry(name.key().into_owned());
        RepoEntryOptional {
            entry,
            env: &self.db_env,
        }
    }
    pub fn filter_by_tag(&mut self, filter: &TagFilterGroup) {
        // TODO: 優化
        log::debug!("根據標籤 {:?} 進行篩選", filter);
        let drain = self.map.drain();
        let mut map = HashMap::default();
        for (key, info) in drain {
            let tags_arr: Vec<_> = info.tags.iter().collect();
            if filter.filter(&tags_arr) {
                log::trace!("腳本 {:?} 通過篩選", info.name);
                map.insert(key, info);
            } else {
                log::trace!("掰掰，{:?}", info.name);
                self.hidden_map.insert(key, info);
            }
        }
        self.map = map;
    }
}
