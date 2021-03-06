use crate::error::Result;
use crate::script::{IntoScriptName, ScriptInfo, ScriptName};
use crate::tag::{Tag, TagFilterGroup};
use crate::Either;
use chrono::{Duration, Utc};
use fxhash::{FxHashMap as HashMap, FxHashSet as HashSet};
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

#[derive(Clone, Debug)]
pub struct DBEnv {
    no_trace: bool,
    info_pool: SqlitePool,
    historian: Historian,
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
            .execute(&self.env.info_pool)
            .await?;
            log::debug!("往資料庫新增腳本成功");
            let id = sqlx::query!("SELECT last_insert_rowid() as id")
                .fetch_one(&self.env.info_pool)
                .await?
                .id;
            log::debug!("得到新腳本 id {}", id);
            info.set_id(id as i64);
        }
        Ok(RepoEntry::new(info, self.env))
    }
}

impl DBEnv {
    pub async fn purge_last_events(&self, id: i64) -> Result {
        log::debug!("清理腳本 {:?} 的最新事件", id);
        sqlx::query!("DELETE FROM last_events WHERE script_id = ?", id)
            .execute(&self.info_pool)
            .await?;
        Ok(())
    }

    async fn update_last_time(&self, info: &ScriptInfo) -> Result {
        let last_time = info.last_time();
        let exec_time = info.exec_time.as_ref().map(|t| **t);
        let exec_done_time = info.exec_done_time.as_ref().map(|t| **t);
        sqlx::query!(
            "INSERT OR REPLACE INTO last_events (script_id, last_time, read, write, exec, exec_done) VALUES(?, ?, ?, ?, ?, ?)",
            info.id,
            last_time,
            *info.read_time,
            *info.write_time,
            exec_time,
            exec_done_time,
        )
        .execute(&self.info_pool)
        .await?;
        Ok(())
    }
    async fn handle_change(&self, info: &ScriptInfo, main_event_id: i64) -> Result<i64> {
        log::debug!("開始修改資料庫 {:?}", info);
        if info.changed {
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

        if self.no_trace {
            return Ok(0);
        }

        let mut last_event_id = 0;

        if let Some(time) = info.exec_done_time.as_ref() {
            if let Some(&code) = time.data() {
                log::debug!("{:?} 的執行完畢事件", info.name);
                last_event_id = self
                    .historian
                    .record(&Event {
                        script_id: info.id,
                        data: EventData::ExecDone {
                            code,
                            main_event_id,
                        },
                        time: **time,
                    })
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
            last_event_id = self
                .historian
                .record(&Event {
                    script_id: info.id,
                    data: EventData::Read,
                    time: *info.read_time,
                })
                .await?;
        }
        if info.write_time.has_changed() {
            log::debug!("{:?} 的寫入事件", info.name);
            last_event_id = self
                .historian
                .record(&Event {
                    script_id: info.id,
                    data: EventData::Write,
                    time: *info.write_time,
                })
                .await?;
        }
        if let Some(time) = info.exec_time.as_ref() {
            if let Some((content, args)) = time.data() {
                log::debug!("{:?} 的執行事件", info.name);
                last_event_id = self
                    .historian
                    .record(&Event {
                        script_id: info.id,
                        data: EventData::Exec { content, args },
                        time: **time,
                    })
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
    known_tags: HashSet<Tag>,
}

impl ScriptRepo {
    pub fn get_db_env(&self) -> &DBEnv {
        &self.db_env
    }

    pub fn iter_known_tags(&self) -> impl Iterator<Item = &Tag> {
        self.known_tags.iter()
    }
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
        no_trace: bool,
    ) -> Result<ScriptRepo> {
        let mut known_tags: HashSet<Tag> = Default::default();

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
        for script in scripts.into_iter() {
            let name = script.name;
            log::trace!("載入腳本：{} {} {}", name, script.ty, script.tags);
            let script_name = name.clone().into_script_name()?;

            let mut builder = ScriptInfo::builder(
                script.id,
                script_name,
                script.ty.into(),
                script.tags.split(',').filter_map(|s| {
                    if s.is_empty() {
                        None
                    } else {
                        // TODO: 錯誤處理，至少印個警告
                        let t: Option<Tag> = s.parse().ok();
                        if let Some(t) = &t {
                            known_tags.insert(t.clone());
                        }
                        t
                    }
                }),
            )
            .created_time(script.created_time);

            if let Some(time) = script.write {
                builder = builder.write_time(time);
            }
            if let Some(time) = script.read {
                builder = builder.read_time(time);
            }
            if let Some(time) = script.exec {
                builder = builder.exec_time(time);
            }
            if let Some(time) = script.exec_done {
                builder = builder.exec_done_time(time);
            }

            let script = builder.build();

            let hide_by_time = if let Some((time_bound, archaeology)) = time_bound {
                let overtime = time_bound > script.last_time_without_read();
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
            known_tags,
            latest_name: None,
            db_env: DBEnv {
                info_pool: pool,
                no_trace,
                historian,
            },
        })
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

    pub async fn remove(&mut self, name: &ScriptName) -> Result {
        if let Some(info) = self.map.remove(&*name.key()) {
            log::debug!("從資料庫刪除腳本 {:?}", info);
            self.db_env.historian.remove(info.id).await?;
            self.db_env.purge_last_events(info.id).await?;
            sqlx::query!("DELETE from script_infos where id = ?", info.id)
                .execute(&self.db_env.info_pool)
                .await?;
        }
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
