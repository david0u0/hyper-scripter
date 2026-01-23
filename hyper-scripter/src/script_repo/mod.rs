use crate::config::Recent;
use crate::error::Result;
use crate::script::{IntoScriptName, ScriptInfo, ScriptName};
use crate::script_type::ScriptType;
use crate::tag::{Tag, TagSelectorGroup};
use chrono::{Duration, NaiveDateTime, Utc};
use fxhash::FxHashMap as HashMap;
use hyper_scripter_historian::{Event, EventData, Historian, LastTimeRecord};
use sqlx::SqlitePool;
use std::collections::hash_map::Entry::{self, *};
use std::fmt::{Display, Formatter, Result as FmtResult};

pub mod helper;
pub use helper::RepoEntry;

#[derive(Clone, Debug, Default)]
pub struct RecentFilter {
    pub recent: Recent,
    pub archaeology: bool,
}
impl Display for RecentFilter {
    fn fmt(&self, w: &mut Formatter<'_>) -> FmtResult {
        write!(w, "{}", self.recent)?;
        if self.archaeology {
            write!(w, " (archaeology)")?;
        }
        Ok(())
    }
}

enum TimeBound {
    Timeless,
    Bound(Option<NaiveDateTime>),
}
impl TimeBound {
    fn new(recent: Recent) -> Self {
        match recent {
            Recent::Timeless => TimeBound::Timeless,
            Recent::NoNeglect => TimeBound::Bound(None),
            Recent::Days(d) => {
                let mut time = Utc::now().naive_utc();
                time -= Duration::days(d.into());
                TimeBound::Bound(Some(time))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Visibility {
    Normal,
    All,
    Inverse,
}
impl Visibility {
    pub fn is_normal(&self) -> bool {
        matches!(self, Self::Normal)
    }
    pub fn is_all(&self) -> bool {
        matches!(self, Self::All)
    }
    pub fn is_inverse(&self) -> bool {
        matches!(self, Self::Inverse)
    }
    pub fn invert(self) -> Self {
        match self {
            Self::Normal => Self::Inverse,
            Self::Inverse => Self::Normal,
            Self::All => {
                log::warn!("無效的可見度反轉：all => all");
                Self::All
            }
        }
    }
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
    info_pool: SqlitePool,
    pub historian: Historian,
    trace_opt: TraceOption,
    modifies_script: bool,
}

pub struct RepoEntryOptional<'b> {
    entry: Entry<'b, String, ScriptInfo>,
    env: &'b DBEnv,
}
impl<'b> RepoEntryOptional<'b> {
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
    pub async fn close(self) {
        futures::join!(self.info_pool.close(), self.historian.close());

        // FIXME: sqlx bug: 這邊可能不會正確關閉，導致有些記錄遺失，暫時解是關閉後加一個 `sleep`
        #[cfg(debug_assertions)]
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
    pub fn new(info_pool: SqlitePool, historian: Historian, modifies_script: bool) -> Self {
        Self {
            info_pool,
            historian,
            modifies_script,
            trace_opt: TraceOption::Normal,
        }
    }
    pub async fn handle_neglect(&self, id: i64) -> Result {
        let time = Utc::now().naive_utc();
        sqlx::query!(
            "
            INSERT OR IGNORE INTO last_events (script_id) VALUES(?);
            UPDATE last_events SET neglect = ? WHERE script_id = ?
            ",
            id,
            time,
            id
        )
        .execute(&self.info_pool)
        .await?;
        Ok(())
    }

    pub async fn update_last_time_directly(&self, last_time: LastTimeRecord) -> Result {
        let LastTimeRecord {
            script_id,
            exec_time,
            exec_done_time,
            humble_time,
        } = last_time;
        sqlx::query!(
            "UPDATE last_events set humble = ?, exec = ?, exec_done = ? WHERE script_id = ?",
            humble_time,
            exec_time,
            exec_done_time,
            script_id
        )
        .execute(&self.info_pool)
        .await?;
        Ok(())
    }
    async fn update_last_time(&self, info: &ScriptInfo) -> Result {
        let exec_count = info.exec_count as i32;
        match self.trace_opt {
            TraceOption::NoTrace => return Ok(()),
            TraceOption::Normal => (),
            TraceOption::Humble => {
                // FIXME: what if a script is created with humble?
                let humble_time = info.last_major_time();
                sqlx::query!(
                    "UPDATE last_events set humble = ?, exec_count = ? WHERE script_id = ?",
                    humble_time,
                    exec_count,
                    info.id,
                )
                .execute(&self.info_pool)
                .await?;
                return Ok(());
            }
        }

        let exec_time = info.exec_time.as_ref().map(|t| **t);
        let exec_done_time = info.exec_done_time.as_ref().map(|t| **t);
        let neglect_time = info.neglect_time.as_ref().map(|t| **t);
        sqlx::query!(
            "
            INSERT OR REPLACE INTO last_events
            (script_id, read, write, exec, exec_done, neglect, humble, exec_count)
            VALUES(?, ?, ?, ?, ?, ?, ?, ?)
            ",
            info.id,
            *info.read_time,
            *info.write_time,
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
        let res = sqlx::query!(
            "
            INSERT INTO script_infos (name, ty, tags, hash)
            VALUES(?, ?, ?, ?)
            RETURNING id
            ",
            name,
            ty,
            tags,
            info.hash,
        )
        .fetch_one(&self.info_pool)
        .await?;
        Ok(res.id)
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
                "UPDATE script_infos SET name = ?, tags = ?, ty = ?, hash = ? where id = ?",
                name,
                tags,
                ty,
                info.hash,
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
        if let Some(time) = info.exec_time.as_ref() {
            if let Some((args, envs, dir)) = time.data() {
                log::debug!("{:?} 的執行事件", info.name);
                last_event_id = record_event!(
                    **time,
                    EventData::Exec {
                        args,
                        envs,
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
    pub time_hidden_count: u32,
    pub recent_filter: RecentFilter,
}

macro_rules! iter_by_vis {
    ($self:expr, $vis:expr) => {{
        let (iter, iter2) = match $vis {
            Visibility::Normal => ($self.map.iter_mut(), None),
            Visibility::All => ($self.map.iter_mut(), Some($self.hidden_map.iter_mut())),
            Visibility::Inverse => ($self.hidden_map.iter_mut(), None),
        };
        iter.chain(iter2.into_iter().flatten()).map(|(_, v)| v)
    }};
}

impl ScriptRepo {
    pub async fn close(self) {
        self.db_env.close().await;
    }
    pub fn iter(&self) -> impl Iterator<Item = &ScriptInfo> {
        self.map.iter().map(|(_, info)| info)
    }
    pub fn iter_mut(&mut self, visibility: Visibility) -> impl Iterator<Item = RepoEntry<'_>> {
        iter_by_vis!(self, visibility).map(|info| RepoEntry::new(info, &self.db_env))
    }
    pub fn historian(&self) -> &Historian {
        &self.db_env.historian
    }
    pub async fn new(
        recent: RecentFilter,
        db_env: DBEnv,
        selector: &TagSelectorGroup,
    ) -> Result<ScriptRepo> {
        let mut hidden_map = HashMap::<String, ScriptInfo>::default();
        let mut map: HashMap<String, ScriptInfo> = Default::default();
        let time_bound = TimeBound::new(recent.recent);

        let scripts = sqlx::query!(
            "SELECT * FROM script_infos si LEFT JOIN last_events le ON si.id = le.script_id"
        )
        .fetch_all(&db_env.info_pool)
        .await?;
        let mut time_hidden_count = 0;
        for record in scripts.into_iter() {
            let name = record.name;
            log::trace!("載入腳本：{} {} {}", name, record.ty, record.tags);
            let script_name = name.clone().into_script_name_unchecked()?; // NOTE: 從資料庫撈出來就別檢查了吧

            let mut builder = ScriptInfo::builder(
                record.id,
                record.hash,
                script_name,
                ScriptType::new_unchecked(record.ty),
                record.tags.split(',').filter_map(|s| {
                    if s.is_empty() {
                        None
                    } else {
                        Some(Tag::new_unchecked(s.to_string()))
                    }
                }),
            );

            builder.created_time(record.created_time);
            builder.exec_count(record.exec_count.unwrap_or_default() as u64);
            if let Some(time) = record.write {
                builder.write_time(time);
            }
            if let Some(time) = record.read {
                builder.read_time(time);
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

            let mut hide = !selector.select(&script.tags, &script.ty);
            if !hide {
                if let Some(neglect) = record.neglect {
                    log::debug!("腳本 {} 曾於 {} 被忽略", script.name, neglect);
                }

                let overtime = match time_bound {
                    TimeBound::Timeless => false,
                    TimeBound::Bound(time_bound) => {
                        let time_bound = std::cmp::max(time_bound, record.neglect);
                        if let Some(time_bound) = time_bound {
                            time_bound > script.last_major_time()
                        } else {
                            false
                        }
                    }
                };
                hide = recent.archaeology ^ overtime;
                if hide {
                    time_hidden_count += 1;
                }
            }

            if hide {
                hidden_map.insert(name, script);
            } else {
                log::trace!("腳本 {:?} 通過篩選", name);
                map.insert(name, script);
            }
        }
        Ok(ScriptRepo {
            map,
            hidden_map,
            latest_name: None,
            recent_filter: recent,
            time_hidden_count,
            db_env,
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
    pub fn latest_mut(&mut self, n: usize, visibility: Visibility) -> Option<RepoEntry<'_>> {
        // if let Some(name) = &self.latest_name {
        //     // FIXME: 一旦 rust nll 進化就修掉這段
        //     if self.map.contains_key(name) {
        //         return self.map.get_mut(name);
        //     }
        //     log::warn!("快取住的最新資訊已經不見了…？重找一次");
        // }
        // self.latest_mut_no_cache()
        let mut v: Vec<_> = iter_by_vis!(self, visibility).collect();
        v.sort_by_key(|s| s.last_time());
        if v.len() >= n {
            let t = v.remove(v.len() - n);
            Some(RepoEntry::new(t, &self.db_env))
        } else {
            None
        }
    }
    pub fn get_mut(&mut self, name: &ScriptName, visibility: Visibility) -> Option<RepoEntry<'_>> {
        // FIXME: 一旦 NLL 進化就修掉這個 unsafe
        let map = &mut self.map as *mut HashMap<String, ScriptInfo>;
        let map = unsafe { &mut *map };
        let key = name.key();
        let info = match visibility {
            Visibility::Normal => map.get_mut(&*key),
            Visibility::Inverse => self.hidden_map.get_mut(&*key),
            Visibility::All => {
                let info = map.get_mut(&*key);
                // 用 Option::or 有一些生命週期的怪問題…
                if info.is_some() {
                    info
                } else {
                    self.hidden_map.get_mut(&*key)
                }
            }
        };
        let env = &self.db_env;
        info.map(move |info| RepoEntry::new(info, env))
    }
    pub fn get_mut_by_id(&mut self, id: i64) -> Option<RepoEntry<'_>> {
        // XXX: 複雜度很瞎
        self.iter_mut(Visibility::All).find(|e| e.id == id)
    }

    pub async fn remove(&mut self, id: i64) -> Result {
        // TODO: 從 map 中刪掉？但如果之後沒其它用途似乎也未必需要...
        log::debug!("從資料庫刪除腳本 {:?}", id);
        self.db_env.handle_delete(id).await?;
        Ok(())
    }
    pub fn entry(&mut self, name: &ScriptName) -> RepoEntryOptional<'_> {
        let entry = self.map.entry(name.key().into_owned());
        RepoEntryOptional {
            entry,
            env: &self.db_env,
        }
    }
    pub fn entry_hidden(&mut self, name: &ScriptName) -> RepoEntryOptional<'_> {
        let entry = self.hidden_map.entry(name.key().into_owned());
        RepoEntryOptional {
            entry,
            env: &self.db_env,
        }
    }
}
