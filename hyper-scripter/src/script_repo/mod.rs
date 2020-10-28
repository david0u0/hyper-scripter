use crate::error::Result;
use crate::path;
use crate::script::{AsScriptName, ScriptInfo, ScriptName};
use crate::tag::{Tag, TagFilterGroup};
use crate::Either;
use async_trait::async_trait;
use chrono::{Duration, NaiveDateTime, Utc};
use hyper_scripter_historian::{Event, EventData, EventType, Historian};
use sqlx::SqlitePool;
use std::collections::hash_map::Entry::{self, *};
use std::collections::HashMap;

pub mod helper;
use helper::*;

#[derive(Clone, Debug)]
pub struct DBEnv {
    info_pool: SqlitePool,
    historian: Historian,
}

pub type ScriptRepoEntry<'a, 'b> = RepoEntry<'a, 'b, DBEnv>;

pub struct ScriptRepoEntryOptional<'a, 'b> {
    entry: Entry<'b, String, ScriptInfo<'a>>,
    env: &'b DBEnv,
}
impl<'a, 'b> ScriptRepoEntryOptional<'a, 'b> {
    pub fn into_either(self) -> Either<ScriptRepoEntry<'a, 'b>, Self> {
        match self.entry {
            Occupied(entry) => Either::One(RepoEntry {
                env: self.env,
                info: entry.into_mut(),
            }),
            _ => Either::Two(self),
        }
    }
    pub async fn or_insert(self, info: ScriptInfo<'a>) -> Result<ScriptRepoEntry<'a, 'b>> {
        let exist = match &self.entry {
            Vacant(_) => false,
            _ => true,
        };
        let info = self.entry.or_insert(info);
        if !exist {
            log::debug!("往資料庫塞新腳本 {:?}", info);
            let name_cow = info.name.key();
            let name = name_cow.as_ref();
            let category = info.ty.as_ref();
            let tags = join_tags(info.tags.iter());
            sqlx::query!(
                "
                INSERT INTO script_infos (name, category, tags)
                VALUES(?, ?, ?)
                ",
                name,
                category,
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
            info.id = id as i64;
        }
        Ok(RepoEntry {
            info,
            env: self.env,
        })
    }
}

#[async_trait]
impl Environment for DBEnv {
    async fn handle_change<'a>(&self, info: &ScriptInfo<'a>) -> Result {
        log::debug!("開始修改資料庫 {:?}", info);
        let name_cow = info.name.key();
        let name = name_cow.as_ref();
        let tags = join_tags(info.tags.iter());
        let category = info.ty.as_ref();
        let write_time = *info.write_time;
        sqlx::query!(
            "UPDATE script_infos SET name = ?, tags = ?, category = ?, write_time = ? where id = ?",
            name,
            tags,
            category,
            write_time,
            info.id,
        )
        .execute(&self.info_pool)
        .await?;

        if info.read_time.has_changed() {
            log::debug!("{:?} 的讀取事件", info.name);
            self.historian
                .record(Event {
                    script_id: info.id,
                    data: EventData::Read,
                })
                .await?;
        }
        if info.miss_time.as_ref().map_or(false, |t| t.has_changed()) {
            log::debug!("{:?} 的錯過事件", info.name);
            self.historian
                .record(Event {
                    script_id: info.id,
                    data: EventData::Miss,
                })
                .await?;
        }
        if let Some(content) = info.exec_time.as_ref().map_or(None, |t| t.data()) {
            log::debug!("{:?} 的執行事件", info.name);
            self.historian
                .record(Event {
                    script_id: info.id,
                    data: EventData::Exec(content),
                })
                .await?;
        }

        Ok(())
    }
}

fn join_tags<'a, I: Iterator<Item = &'a Tag>>(tags: I) -> String {
    let tags_arr: Vec<&str> = tags.map(|t| t.as_ref()).collect();
    tags_arr.join(",")
}

#[derive(Debug)]
pub struct ScriptRepo<'a> {
    map: HashMap<String, ScriptInfo<'a>>,
    hidden_map: HashMap<String, ScriptInfo<'a>>,
    latest_name: Option<String>,
    db_env: DBEnv,
}

impl<'a> ScriptRepo<'a> {
    pub fn iter(&self) -> impl Iterator<Item = &ScriptInfo> {
        self.map.iter().map(|(_, info)| info)
    }
    pub fn iter_mut<'b>(&'b mut self) -> Iter<'a, 'b, DBEnv> {
        Iter {
            iter: self.map.iter_mut(),
            env: &self.db_env,
        }
    }
    pub fn iter_hidden_mut<'b>(&'b mut self) -> Iter<'a, 'b, DBEnv> {
        Iter {
            iter: self.hidden_map.iter_mut(),
            env: &self.db_env,
        }
    }
    pub fn historian(&self) -> &Historian {
        &self.db_env.historian
    }
    pub async fn new<'b>(pool: SqlitePool, recent: Option<u32>) -> Result<ScriptRepo<'b>> {
        let historian = Historian::new(path::get_home()).await?;

        let mut hidden_map = HashMap::<String, ScriptInfo>::new();
        let time_bound = recent.map(|recent| {
            let mut time = Utc::now().naive_utc();
            time -= Duration::days(recent.into());
            time
        });

        let scripts = sqlx::query!("SELECT * from script_infos ORDER BY id")
            .fetch_all(&pool)
            .await?;
        let last_read_records = historian.last_time_of(EventType::Read).await?;
        let last_exec_records = historian.last_time_of(EventType::Exec).await?;
        let last_miss_records = historian.last_time_of(EventType::Miss).await?;
        let mut last_read: &[_] = &last_read_records;
        let mut last_exec: &[_] = &last_exec_records;
        let mut last_miss: &[_] = &last_miss_records;
        let mut map: HashMap<String, ScriptInfo> = Default::default();
        for script in scripts.into_iter() {
            use std::str::FromStr;

            let name = script.name;
            log::trace!("載入腳本：{} {} {}", name, script.category, script.tags);
            let script_name = name.as_script_name()?.into_static(); // TODO: 正確實作 from string

            let mut builder = ScriptInfo::builder(
                script.id,
                script_name,
                script.category.into(),
                script.tags.split(",").filter_map(|s| {
                    if s == "" {
                        None
                    } else {
                        // TODO: 錯誤處理，至少印個警告
                        Tag::from_str(s).ok()
                    }
                }),
            )
            .created_time(script.created_time)
            .write_time(script.write_time);

            if let Some(time) = extract_from_time(script.id, &mut last_miss) {
                builder = builder.miss_time(time);
            }
            if let Some(time) = extract_from_time(script.id, &mut last_exec) {
                builder = builder.exec_time(time);
            }
            if let Some(time) = extract_from_time(script.id, &mut last_read) {
                builder = builder.read_time(time);
            } else {
                log::warn!(
                    "找不到 {:?} 的讀取時間，可能是資料庫爛了，改用創建時間",
                    builder.name
                );
                builder = builder.read_time(script.created_time);
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
    pub fn latest_mut(&mut self, n: usize) -> Option<ScriptRepoEntry<'a, '_>> {
        // if let Some(name) = &self.latest_name {
        //     // FIXME: 一旦 rust nll 進化就修掉這段
        //     if self.map.contains_key(name) {
        //         return self.map.get_mut(name);
        //     }
        //     log::warn!("快取住的最新資訊已經不見了…？重找一次");
        // }
        // self.latest_mut_no_cache()
        let mut v: Vec<_> = self.map.iter_mut().map(|(_, s)| s).collect();
        v.sort_by_key(|s| s.last_time());
        if v.len() >= n {
            // SAFETY: 從向量中讀一個可變指針安啦
            let t = unsafe { std::ptr::read(&v[v.len() - n]) };
            Some(RepoEntry {
                info: t,
                env: &self.db_env,
            })
        } else {
            None
        }
    }
    pub fn get_mut(&mut self, name: &ScriptName) -> Option<ScriptRepoEntry<'a, '_>> {
        match self.map.get_mut(&*name.key()) {
            None => None,
            Some(info) => Some(RepoEntry {
                info,
                env: &self.db_env,
            }),
        }
    }
    pub fn get_hidden_mut(&mut self, name: &ScriptName) -> Option<ScriptRepoEntry<'a, '_>> {
        match self.hidden_map.get_mut(&*name.key()) {
            None => None,
            Some(info) => Some(RepoEntry {
                info,
                env: &self.db_env,
            }),
        }
    }
    pub fn get_regardless_mut(&mut self, name: &ScriptName) -> Option<ScriptRepoEntry<'a, '_>> {
        // FIXME: 一旦 NLL 進化就修掉這段，改用 if let Some(..) = get_mut { } else { get_hidden_mut... }
        match self.map.get_mut(&*name.key()) {
            Some(info) => {
                return Some(RepoEntry {
                    info,
                    env: &self.db_env,
                })
            }
            _ => (),
        };
        match self.hidden_map.get_mut(&*name.key()) {
            None => None,
            Some(info) => Some(RepoEntry {
                info,
                env: &self.db_env,
            }),
        }
    }
    pub async fn remove<'c>(&mut self, name: &ScriptName<'c>) -> Result {
        if let Some(info) = self.map.remove(&*name.key()) {
            log::debug!("從資料庫刪除腳本 {:?}", info);
            sqlx::query!("DELETE from script_infos where id = ?", info.id)
                .execute(&self.db_env.info_pool)
                .await?;
        }
        Ok(())
    }
    pub fn entry<'z>(&mut self, name: &ScriptName<'z>) -> ScriptRepoEntryOptional<'a, '_> {
        // TODO: 決定要插 hidden 與否
        let entry = self.map.entry(name.key().into_owned());
        ScriptRepoEntryOptional {
            entry,
            env: &self.db_env,
        }
    }
    pub fn filter_by_tag(&mut self, filter: &TagFilterGroup) {
        // TODO: 優化
        log::debug!("根據標籤 {:?} 進行篩選", filter);
        let drain = self.map.drain();
        let mut map = HashMap::new();
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
                if *id == cur_id {
                    *series = &series[1..series.len()];
                    return Some(*time);
                } else if *id < cur_id {
                    *series = &series[1..series.len()];
                } else {
                    return None;
                }
            }
            None => {
                return None;
            }
        }
    }
}
