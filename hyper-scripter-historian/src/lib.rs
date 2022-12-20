use chrono::NaiveDateTime;
use sqlx::migrate::MigrateError;
use sqlx::{error::Error as DBError, Pool, Sqlite, SqlitePool};
use std::num::NonZeroU64;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

mod db;
mod event;
pub mod migration;
pub use event::*;

const ZERO: i64 = 0;
const EMPTY_STR: &str = "";

const EXEC_CODE: i8 = EventType::Exec.get_code();
const EXEC_DONE_CODE: i8 = EventType::ExecDone.get_code();

#[derive(Debug, Clone)]
pub struct Historian {
    pool: Arc<RwLock<SqlitePool>>,
    file_path: PathBuf,
}

async fn raw_record_event(pool: &Pool<Sqlite>, event: DBEvent<'_>) -> Result<i64, DBError> {
    let res = sqlx::query!(
        "
        INSERT INTO events
        (script_id, type, cmd, args, content, time, main_event_id, dir, envs, humble)
        VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING id
        ",
        event.script_id,
        event.ty,
        event.cmd,
        event.args,
        event.content,
        event.time,
        event.main_event_id,
        event.dir,
        event.envs,
        event.humble
    )
    .fetch_one(pool)
    .await?;
    Ok(res.id)
}

#[derive(Clone, Copy)]
struct DBEvent<'a> {
    script_id: i64,
    ty: i8,
    cmd: &'a str,
    time: NaiveDateTime,
    args: Option<&'a str>,
    dir: Option<&'a str>,
    envs: Option<&'a str>,
    content: Option<&'a str>,
    humble: bool,
    main_event_id: i64,
}
impl<'a> DBEvent<'a> {
    fn new(script_id: i64, time: NaiveDateTime, ty: i8, cmd: &'a str, humble: bool) -> Self {
        DBEvent {
            script_id,
            time,
            ty,
            cmd,
            humble,
            main_event_id: ZERO,
            envs: None,
            content: None,
            args: None,
            dir: None,
        }
    }
    fn args(mut self, value: &'a str) -> Self {
        self.args = Some(value);
        self
    }
    fn dir(mut self, value: &'a str) -> Self {
        self.dir = Some(value);
        self
    }
    fn content(mut self, value: &'a str) -> Self {
        self.content = Some(value);
        self
    }
    fn envs(mut self, value: &'a str) -> Self {
        self.envs = Some(value);
        self
    }
    fn humble(mut self) -> Self {
        self.humble = true;
        self
    }
    fn main_event_id(mut self, value: i64) -> Self {
        self.main_event_id = value;
        self
    }
}

macro_rules! last_arg {
    ($select:literal, $offset:expr, $limit:expr, $group_by:literal, $where:literal $(+ $more_where:literal)* , $($var:expr),*) => {{
        sqlx::query!(
            "
            WITH args AS (
                SELECT " + $select + ", max(time) as time FROM events
                WHERE type = ? AND NOT ignored "
                +
                $where
                $(+ $more_where)*
                +
                " GROUP BY args, script_id " + $group_by + " ORDER BY time DESC LIMIT ? OFFSET ?
            ) SELECT "
                + $select
                + " FROM args
            ",
            EXEC_CODE,
            $($var, )*
            $limit,
            $offset,
        )
    }};
}
macro_rules! do_last_arg {
    ($select:literal, $maybe_envs:literal, $ids:expr, $limit:expr, $offset:expr, $no_humble:expr, $dir:expr, $historian:expr) => {{
        let ids = join_id_str($ids);
        log::info!("查詢歷史 {}", ids);
        let limit = $limit as i64;
        let offset = $offset as i64;
        let no_dir = $dir.is_none();
        let dir = $dir.map(|p| p.to_string_lossy());
        let dir = dir.as_deref().unwrap_or(EMPTY_STR);
        // FIXME: 一旦可以綁定陣列就換掉這個醜死人的 instr
        last_arg!(
            $select,
            offset,
            limit,
            $maybe_envs,
            "
            AND instr(?, '[' || script_id || ']') > 0 AND (? OR dir = ?)
            AND (NOT ? OR NOT humble)
            ",
            ids,
            no_dir,
            dir,
            $no_humble
        )
        .fetch_all(&*$historian.pool.read().unwrap())
        .await
    }};
}

macro_rules! ignore_or_humble_arg {
    ($ignore_or_humble:literal, $pool:expr, $cond:literal $(+ $more_cond:literal)*, $($var:expr),+) => {
        sqlx::query!(
            "
            UPDATE events SET " + $ignore_or_humble + " = true
            WHERE type = ? AND main_event_id IN (
                SELECT id FROM events WHERE type = ? AND NOT ignored AND "
                + $cond $(+ $more_cond)*
                + "
            )
            ",
            EXEC_DONE_CODE,
            EXEC_CODE,
            $($var),*
        )
        .execute(&*$pool)
        .await?;

        sqlx::query!(
            "
            UPDATE events SET " + $ignore_or_humble + " = true
            WHERE type = ? AND NOT ignored AND
            "
                + $cond $(+ $more_cond)*,
            EXEC_CODE,
            $($var),*
        )
        .execute(&*$pool)
        .await?;
    };
}

#[derive(Debug)]
pub struct LastTimeRecord {
    pub script_id: i64,
    pub exec_time: Option<NaiveDateTime>,
    pub exec_done_time: Option<NaiveDateTime>,
    pub humble_time: Option<NaiveDateTime>,
}

impl Historian {
    pub async fn close(self) {
        log::info!("close the historian database");
        if let Ok(pool) = self.pool.read() {
            pool.close().await;
        }
    }
    async fn raw_record(&self, event: DBEvent<'_>) -> Result<i64, DBError> {
        let pool = &mut *self.pool.write().unwrap();
        let res = raw_record_event(pool, event).await;
        if res.is_err() {
            pool.close().await;
            log::warn!("資料庫錯誤 {:?}，再試最後一次！", res);
            *pool = db::get_pool(&self.file_path).await?;
            return raw_record_event(pool, event).await;
        }

        res
    }
    pub async fn new(file_path: PathBuf) -> Result<Self, DBError> {
        db::get_pool(&file_path).await.map(|pool| Historian {
            pool: Arc::new(RwLock::new(pool)),
            file_path,
        })
    }
    pub async fn do_migrate(file: impl AsRef<Path>) -> Result<(), MigrateError> {
        migration::do_migrate(file.as_ref()).await?;
        Ok(())
    }

    pub async fn remove(&self, script_id: i64) -> Result<(), DBError> {
        let pool = self.pool.read().unwrap();
        sqlx::query!("DELETE FROM events WHERE script_id = ?", script_id,)
            .execute(&*pool)
            .await?;
        Ok(())
    }

    pub async fn record(&self, event: &Event<'_>) -> Result<i64, DBError> {
        log::debug!("記錄事件 {:?}", event);
        let ty = event.data.get_type().get_code();
        let cmd = std::env::args().collect::<Vec<_>>().join(" ");
        let mut db_event = DBEvent::new(event.script_id, event.time, ty, &cmd, event.humble);
        let id = match &event.data {
            EventData::Write | EventData::Read | EventData::Miss => {
                self.raw_record(db_event).await?
            }
            EventData::Exec {
                content,
                args,
                envs,
                dir,
            } => {
                let mut content = Some(*content);
                let last_event = sqlx::query!(
                    "
                    SELECT content FROM events
                    WHERE type = ? AND script_id = ? AND NOT content IS NULL
                    ORDER BY time DESC LIMIT 1
                    ",
                    ty,
                    event.script_id
                )
                .fetch_optional(&*self.pool.read().unwrap())
                .await?;
                if let Some(last_event) = last_event {
                    if last_event.content.as_deref() == content {
                        log::debug!("上次執行內容相同，不重複記錄");
                        content = None;
                    }
                }
                db_event.content = content;
                let dir = dir.map(|p| p.to_string_lossy()).unwrap_or_default();
                self.raw_record(db_event.envs(envs).dir(dir.as_ref()).args(args))
                    .await?
            }
            EventData::ExecDone {
                code,
                main_event_id,
            } => {
                let main_event = sqlx::query!(
                    "SELECT ignored, humble FROM events WHERE type = ? AND id = ?",
                    EXEC_CODE,
                    main_event_id
                )
                .fetch_optional(&*self.pool.read().unwrap())
                .await?;
                let main_event = match main_event {
                    Some(e) => e,
                    None => {
                        log::warn!("找不到主要事件，可能被 tidy 掉了");
                        return Ok(ZERO);
                    }
                };
                if main_event.ignored {
                    return Ok(ZERO);
                } else if main_event.humble {
                    log::debug!("謙卑地執行完畢了");
                    db_event = db_event.humble();
                }

                let code = code.to_string();
                let id = self
                    .raw_record(db_event.content(&code).main_event_id(*main_event_id))
                    .await?;

                if db_event.humble {
                    // XXX: 用很怪異的方式告訴外面的人不要記錄最新時間，醜死
                    ZERO
                } else {
                    id
                }
            }
        };
        Ok(id)
    }

    pub async fn previous_args(
        &self,
        id: i64,
        dir: Option<&Path>,
    ) -> Result<Option<(String, String)>, DBError> {
        let no_dir = dir.is_none();
        let dir = dir.map(|p| p.to_string_lossy());
        let dir = dir.as_deref().unwrap_or(EMPTY_STR);
        let res = sqlx::query!(
            "
            SELECT args, envs FROM events
            WHERE type = ? AND script_id = ? AND NOT ignored
            AND (? OR dir = ?)
            ORDER BY time DESC LIMIT 1
            ",
            EXEC_CODE,
            id,
            no_dir,
            dir
        )
        .fetch_optional(&*self.pool.read().unwrap())
        .await?;
        Ok(res.map(|res| (res.args.unwrap_or_default(), res.envs.unwrap_or_default())))
    }

    pub async fn previous_args_list(
        &self,
        ids: &[i64],
        limit: u32,
        offset: u32,
        no_humble: bool,
        dir: Option<&Path>,
    ) -> Result<impl ExactSizeIterator<Item = (i64, String)>, DBError> {
        let res = do_last_arg!(
            "script_id, args",
            "",
            ids,
            limit,
            offset,
            no_humble,
            dir,
            self
        )?;
        Ok(res
            .into_iter()
            .map(|res| (res.script_id, res.args.unwrap_or_default())))
    }

    pub async fn previous_args_list_with_envs(
        &self,
        ids: &[i64],
        limit: u32,
        offset: u32,
        no_humble: bool,
        dir: Option<&Path>,
    ) -> Result<impl ExactSizeIterator<Item = (i64, String, String)>, DBError> {
        let res = do_last_arg!(
            "script_id, args, envs",
            ", envs",
            ids,
            limit,
            offset,
            no_humble,
            dir,
            self
        )?;
        Ok(res.into_iter().map(|res| {
            (
                res.script_id,
                res.args.unwrap_or_default(),
                res.envs.unwrap_or_default(),
            )
        }))
    }

    async fn make_last_time_record(&self, script_id: i64) -> Result<LastTimeRecord, DBError> {
        let res = sqlx::query_as_unchecked!(
            LastTimeRecord,
            "
            SELECT
                ? as script_id,
                (SELECT time FROM events
                WHERE script_id = ? AND NOT ignored AND humble
                ORDER BY time DESC LIMIT 1) as humble_time,
                (SELECT time FROM events
                WHERE script_id = ? AND NOT ignored AND NOT humble AND type = ?
                ORDER BY time DESC LIMIT 1) as exec_time,
                (SELECT time FROM events
                WHERE script_id = ? AND NOT ignored AND NOT humble AND type = ?
                ORDER BY time DESC LIMIT 1) as exec_done_time
            ",
            script_id,
            script_id,
            script_id,
            EXEC_CODE,
            script_id,
            EXEC_DONE_CODE
        )
        .fetch_one(&*self.pool.read().unwrap())
        .await?;

        Ok(LastTimeRecord {
            script_id,
            exec_time: res.exec_time,
            exec_done_time: res.exec_done_time,
            humble_time: res.humble_time,
        })
    }
    pub async fn ignore_args_by_id(
        &self,
        event_id: NonZeroU64,
    ) -> Result<Option<LastTimeRecord>, DBError> {
        self.process_args_by_id(false, event_id).await
    }
    pub async fn humble_args_by_id(
        &self,
        event_id: NonZeroU64,
    ) -> Result<Option<LastTimeRecord>, DBError> {
        self.process_args_by_id(true, event_id).await
    }
    /// humble or ignore
    async fn process_args_by_id(
        &self,
        is_humble: bool,
        event_id: NonZeroU64,
    ) -> Result<Option<LastTimeRecord>, DBError> {
        let pool = self.pool.read().unwrap();
        let event_id = event_id.get() as i64;
        let latest_record = sqlx::query!(
            "
            SELECT id, script_id FROM events
            WHERE type = ? AND script_id = (SELECT script_id FROM events WHERE id = ?)
            ORDER BY time DESC LIMIT 1
            ",
            EXEC_CODE,
            event_id,
        )
        .fetch_one(&*pool)
        .await?;
        // TODO: check if this event is exec?

        if is_humble {
            ignore_or_humble_arg!("humble", pool, "id = ?", event_id);
        } else {
            ignore_or_humble_arg!("ignored", pool, "id = ?", event_id);
        }

        if latest_record.id == event_id {
            // NOTE: 若 event_id 為最新但已被 ignored/humble，仍會被抓成 last_record 並進入這裡
            // 但應該不致於有太大的效能問題
            log::info!("process last args");
            let ret = self.make_last_time_record(latest_record.script_id).await?;
            return Ok(Some(ret));
        }
        Ok(None)
    }
    pub async fn ignore_args_range(
        &self,
        ids: &[i64],
        dir: Option<&Path>,
        no_humble: bool,
        show_env: bool,
        min: NonZeroU64,
        max: Option<NonZeroU64>,
    ) -> Result<Vec<LastTimeRecord>, DBError> {
        let ids_str = join_id_str(ids);

        let offset = min.get() as i64 - 1;
        let limit = if let Some(max) = max {
            (max.get() - min.get()) as i64
        } else {
            -1
        };
        let no_dir = dir.is_none();
        let dir = dir.map(|p| p.to_string_lossy());
        let dir = dir.as_deref().unwrap_or(EMPTY_STR);
        log::info!("忽略歷史 {} {} {}", offset, limit, ids_str);

        let pool = self.pool.read().unwrap();
        macro_rules! ignore_arg {
            ($($target:literal)*) => {{
                // NOTE: 我們知道 script_id || args 串接起來必然是唯一的（因為 args 的格式為 [...]）
                // FIXME: 一旦可以綁定陣列就換掉這個醜死人的 instr
                ignore_or_humble_arg!(
                    "ignored",
                    pool,
                    "
                    (? OR dir == ?) AND
                    (script_id || args " $(+ "||" + $target)* + ") IN (
                        WITH records AS (
                            SELECT max(time) as time, args, script_id " $(+ "," + $target)* +" FROM events
                            WHERE instr(?, '[' || script_id || ']') > 0
                            AND type = ? AND NOT ignored
                            AND (NOT ? OR NOT humble)
                            GROUP BY args, script_id " $( + "," + $target)* + " ORDER BY time DESC LIMIT ? OFFSET ?
                        ) SELECT script_id || args " $(+ "||" + $target)* + " as t FROM records
                    )
                    ",
                    no_dir,
                    dir,
                    ids_str,
                    EXEC_CODE,
                    no_humble,
                    limit,
                    offset
                );
            }};
        }

        if show_env {
            ignore_arg!("envs");
        } else {
            ignore_arg!();
        }

        log::info!("ignore last args");
        let mut ret = vec![];
        for &id in ids {
            // TODO: 平行？
            ret.push(self.make_last_time_record(id).await?);
        }
        Ok(ret)
    }

    pub async fn amend_args_by_id(
        &self,
        event_id: NonZeroU64,
        args: &str,
        envs: Option<&str>,
    ) -> Result<(), DBError> {
        let event_id = event_id.get() as i64;

        macro_rules! amend {
            ($($set:literal, $var:expr),*) => {{
                sqlx::query!(
                    "UPDATE events SET ignored = false, args = ?"
                    + $( "," + $set + "=? " +)*
                    "WHERE type = ? AND id = ? ",
                    args,
                    $($var,)*
                    EXEC_CODE,
                    event_id,
                )
                .execute(&*self.pool.read().unwrap())
                .await?
            }}
        }
        if let Some(envs) = envs {
            amend!("envs", envs);
        } else {
            amend!();
        }
        Ok(())
    }

    /// 除了輸入進來的 script id 外，其它事件通通砍除
    pub async fn clear_except_script_ids(&self, script_ids: &[i64]) -> Result<(), DBError> {
        let ids = join_id_str(script_ids);
        let pool = self.pool.read().unwrap();
        // FIXME: 一旦可以綁定陣列就換掉這個醜死人的 instr
        sqlx::query!(
            "
            DELETE FROM events
            WHERE instr(?, '[' || script_id || ']') <= 0
            ",
            ids
        )
        .execute(&*pool)
        .await?;

        sqlx::query!("VACUUM").execute(&*pool).await?;

        Ok(())
    }

    pub async fn tidy(&self, script_id: i64) -> Result<(), DBError> {
        let pool = self.pool.read().unwrap();
        // XXX: 笑死這啥鬼
        sqlx::query!(
            "
            DELETE FROM events
            WHERE script_id = ?
              AND id NOT IN (
                SELECT
                  (
                    SELECT id FROM events
                    WHERE script_id = ?
                      AND args = e.args
                      AND dir = e.dir
                    ORDER BY time DESC
                    LIMIT 1
                  )
                FROM
                  (
                    SELECT distinct args, dir
                    FROM events
                    WHERE script_id = ?
                      AND NOT ignored
                      AND type = ?
                  ) e
              )
            ",
            script_id,
            script_id,
            script_id,
            EXEC_CODE,
        )
        .execute(&*pool)
        .await?;

        Ok(())
    }
}

fn join_id_str(ids: &[i64]) -> String {
    use std::fmt::Write;
    let mut ret = String::new();
    for id in ids {
        write!(ret, "[{}]", id).unwrap();
    }
    ret
}
