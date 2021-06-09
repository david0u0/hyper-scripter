use crate::error::Result;
use crate::path;
use crate::script::{IntoScriptName, ScriptInfo, ScriptName};
use crate::tag::{Tag, TagFilterGroup};
use crate::Either;
use chrono::{Duration, NaiveDateTime, Utc};
use fxhash::{FxHashMap as HashMap, FxHashSet as HashSet};
use hyper_scripter_historian::{Event, EventData, EventType, Historian};
use sqlx::SqlitePool;
use std::collections::hash_map::Entry::{self, *};

pub mod helper;
pub use helper::RepoEntry;
use helper::*;

#[derive(Clone, Debug)]
pub struct DBEnv {
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
    async fn handle_change(&self, info: &ScriptInfo) -> Result<i64> {
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

        let mut last_event_id = 0;

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
    pub async fn new(pool: SqlitePool, recent: Option<u32>) -> Result<ScriptRepo> {
        let historian = Historian::new(path::get_home()).await?;
        let mut known_tags: HashSet<Tag> = Default::default();

        let mut hidden_map = HashMap::<String, ScriptInfo>::default();
        let time_bound = recent.map(|recent| {
            let mut time = Utc::now().naive_utc();
            time -= Duration::days(recent.into());
            time
        });

        let scripts = sqlx::query!("SELECT * from script_infos ORDER BY id")
            .fetch_all(&pool)
            .await?;
        let last_read_records = historian.last_time_of(EventType::Read).await?;
        let last_write_records = historian.last_time_of(EventType::Write).await?;
        let last_exec_records = historian.last_time_of(EventType::Exec).await?;
        let last_exec_done_records = historian.last_time_of(EventType::ExecDone).await?;
        let mut last_read: &[_] = &last_read_records;
        let mut last_write: &[_] = &last_write_records;
        let mut last_exec: &[_] = &last_exec_records;
        let mut last_exec_done: &[_] = &last_exec_done_records;
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

            if let Some(time) = extract_from_time(script.id, &mut last_exec) {
                builder = builder.exec_time(time);
            }
            if let Some(time) = extract_from_time(script.id, &mut last_exec_done) {
                builder = builder.exec_done_time(time);
            }
            if let Some(time) = extract_from_time(script.id, &mut last_read) {
                builder = builder.read_time(time);
            } else {
                log::warn!(
                    "找不到 {:?} 的讀取時間，可能是資料庫爛了，改用創建時間",
                    builder.name
                );
            }
            if let Some(time) = extract_from_time(script.id, &mut last_write) {
                builder = builder.write_time(time);
            } else {
                log::warn!(
                    "找不到 {:?} 的寫入時間，可能是資料庫爛了，改用創建時間",
                    builder.name
                );
            }

            let script = builder.build();
            if time_bound.map_or(true, |time_bound| script.last_time() > time_bound) {
                map.insert(name, script);
            } else {
                hidden_map.insert(name, script);
            }
        }
        Ok(ScriptRepo {
            map,
            hidden_map,
            known_tags,
            latest_name: None,
            db_env: DBEnv {
                info_pool: pool,
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

fn extract_from_time(cur_id: i64, series: &mut &[(i64, NaiveDateTime)]) -> Option<NaiveDateTime> {
    loop {
        match series.first() {
            Some((id, time)) => {
                use std::cmp::Ordering;
                match id.cmp(&cur_id) {
                    Ordering::Equal => {
                        *series = &series[1..series.len()];
                        return Some(*time);
                    }
                    Ordering::Less => *series = &series[1..series.len()],
                    Ordering::Greater => return None,
                }
            }
            None => {
                return None;
            }
        }
    }
}
