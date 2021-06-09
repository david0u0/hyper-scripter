#[macro_use]
extern crate derive_more;

use chrono::NaiveDateTime;
use sqlx::{error::Error as DBError, Pool, Sqlite, SqlitePool};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

mod db;
mod event;
pub mod migration;
pub use event::*;

#[derive(Debug, Clone)]
pub struct Historian {
    pool: Arc<RwLock<SqlitePool>>,
    path: PathBuf,
}

async fn ignore_last_args(pool: &Pool<Sqlite>, script_id: i64, ty: String) -> Result<(), DBError> {
    let ty = ty.to_string();
    sqlx::query!(
        "DELETE FROM last_events WHERE script_id = ? AND type = ?",
        script_id,
        ty,
    )
    .execute(pool)
    .await?;

    let last_event = sqlx::query!(
        "
        SELECT * FROM events
        WHERE type = ? AND script_id = ? AND NOT ignored
        ORDER BY time DESC LIMIT 1
        ",
        ty,
        script_id
    )
    .fetch_optional(pool)
    .await?;

    if let Some(last_event) = last_event {
        sqlx::query!(
            "
            INSERT OR REPLACE INTO last_events (script_id, type, cmd, args, content, time)
            VALUES(?, ?, ?, ?, ?, ?)
            ",
            script_id,
            ty,
            last_event.cmd,
            last_event.args,
            last_event.content,
            last_event.time
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}
async fn raw_record_event(pool: &Pool<Sqlite>, event: DBEvent<'_>) -> Result<i64, DBError> {
    sqlx::query!(
        "
        INSERT INTO events
        (script_id, type, cmd, args, content, time, main_event_id)
        VALUES(?, ?, ?, ?, ?, ?, ?)
        ",
        event.script_id,
        event.ty,
        event.cmd,
        event.args,
        event.content,
        event.time,
        event.main_event_id
    )
    .execute(pool)
    .await?;
    let res = sqlx::query!("SELECT last_insert_rowid() AS id")
        .fetch_one(pool)
        .await?;
    Ok(res.id as i64)
}
async fn raw_record_last(pool: &Pool<Sqlite>, event: DBEvent<'_>) -> Result<(), DBError> {
    sqlx::query!(
        "INSERT OR REPLACE INTO last_events (script_id, type, cmd, args, content, time) VALUES(?, ?, ?, ?, ?, ?)",
        event.script_id,
        event.ty,
        event.cmd,
        event.args,
        event.content,
        event.time
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Clone, Copy)]
struct DBEvent<'a> {
    script_id: i64,
    ty: &'a str,
    cmd: &'a str,
    time: NaiveDateTime,
    args: Option<&'a str>,
    content: Option<&'a str>,
    main_event_id: i64,
}
impl<'a> DBEvent<'a> {
    fn new(script_id: i64, time: NaiveDateTime, ty: &'a str, cmd: &'a str) -> Self {
        DBEvent {
            script_id,
            time,
            ty,
            cmd,
            main_event_id: 0,
            content: None,
            args: None,
        }
    }
    fn args(mut self, value: &'a str) -> Self {
        self.args = Some(value);
        self
    }
    fn content(mut self, value: &'a str) -> Self {
        self.content = Some(value);
        self
    }
    fn main_event_id(mut self, value: i64) -> Self {
        self.main_event_id = value;
        self
    }
}

impl Historian {
    async fn raw_record(&self, event: DBEvent<'_>) -> Result<i64, DBError> {
        let pool = &mut *self.pool.write().unwrap();
        let res = raw_record_last(pool, event).await;
        if res.is_err() {
            log::warn!("資料庫錯誤 {:?}，再試最後一次！", res);
            *pool = db::get_pool(&self.path).await?;
            raw_record_last(pool, event).await?;
            return raw_record_event(pool, event).await;
        }

        raw_record_event(pool, event).await
    }
    pub async fn new(path: impl AsRef<Path>) -> Result<Self, DBError> {
        let path = path.as_ref().to_owned();
        db::get_pool(&path).await.map(|pool| Historian {
            pool: Arc::new(RwLock::new(pool)),
            path,
        })
    }

    pub async fn remove(&self, script_id: i64) -> Result<(), DBError> {
        let pool = self.pool.read().unwrap();
        sqlx::query!("DELETE FROM last_events WHERE script_id = ?", script_id,)
            .execute(&*pool)
            .await?;
        sqlx::query!("DELETE FROM events WHERE script_id = ?", script_id,)
            .execute(&*pool)
            .await?;
        Ok(())
    }

    pub async fn record(&self, event: &Event<'_>) -> Result<i64, DBError> {
        log::debug!("記錄事件 {:?}", event);
        let ty = event.data.get_type().to_string();
        let time = event.time;
        let cmd = std::env::args().collect::<Vec<_>>().join(" ");
        let mut db_event = DBEvent::new(event.script_id, time, &ty, &cmd);
        let id = match &event.data {
            EventData::Write | EventData::Read => self.raw_record(db_event).await?,
            EventData::Exec { content, args } => {
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
                self.raw_record(db_event.args(args)).await?
            }
            EventData::ExecDone {
                code,
                main_event_id,
            } => {
                let exec_ty = EventType::Exec.to_string();
                let ignored_res = sqlx::query!(
                    "SELECT ignored FROM events WHERE type = ? AND id = ?",
                    exec_ty,
                    main_event_id
                )
                .fetch_one(&*self.pool.read().unwrap())
                .await?;
                if ignored_res.ignored {
                    return Ok(0);
                }
                let code = code.to_string();
                self.raw_record(db_event.content(&code).main_event_id(*main_event_id))
                    .await?
            }
        };
        Ok(id)
    }

    // XXX: 其實可以只回迭代器
    pub async fn last_time_of(&self, ty: EventType) -> Result<Vec<(i64, NaiveDateTime)>, DBError> {
        let ty = ty.to_string();
        let times = sqlx::query!(
            "SELECT script_id, time as time FROM last_events WHERE type = ? ORDER BY script_id",
            ty
        )
        .fetch_all(&*self.pool.read().unwrap())
        .await?;
        Ok(times.into_iter().map(|d| (d.script_id, d.time)).collect())
    }

    pub async fn last_args(&self, id: i64) -> Result<Option<String>, DBError> {
        let ty = EventType::Exec.to_string();
        let res = sqlx::query!(
            "
            SELECT args FROM events
            WHERE type = ? AND script_id = ? AND NOT ignored
            ORDER BY time DESC LIMIT 1
            ",
            ty,
            id
        )
        .fetch_optional(&*self.pool.read().unwrap())
        .await?;
        Ok(res.map(|res| res.args.unwrap_or_default()))
    }

    pub async fn last_args_list(
        &self,
        id: i64,
        limit: u32,
        offset: u32,
    ) -> Result<impl ExactSizeIterator<Item = String>, DBError> {
        let limit = limit as i64;
        let offset = offset as i64;
        let ty = EventType::Exec.to_string();
        let res = sqlx::query!(
            "
            WITH args AS (
                SELECT args, max(time) as time FROM events
                WHERE type = ? AND script_id = ? AND NOT ignored
                GROUP BY args
                ORDER BY time DESC LIMIT ? OFFSET ?
            ) SELECT args FROM args
            ",
            ty,
            id,
            limit,
            offset
        )
        .fetch_all(&*self.pool.read().unwrap())
        .await?;
        Ok(res.into_iter().map(|res| res.args.unwrap_or_default()))
    }

    pub async fn ignore_args(
        &self,
        script_id: i64,
        number: std::num::NonZeroU64,
    ) -> Result<(), DBError> {
        let number = number.get();
        let exec_ty = EventType::Exec.to_string();
        let done_ty = EventType::ExecDone.to_string();
        let offset = number as i64 - 1;
        let pool = self.pool.read().unwrap();
        let args = sqlx::query!(
            "
            WITH args AS (
                SELECT args, max(time) as time FROM events
                WHERE type = ? AND script_id = ? AND NOT ignored
                GROUP BY args
                ORDER BY time DESC LIMIT 1 OFFSET ?
            ) SELECT args FROM args
            ",
            exec_ty,
            script_id,
            offset,
        )
        .fetch_one(&*pool)
        .await?
        .args;

        sqlx::query!(
            "
            UPDATE events SET ignored = true
            WHERE script_id = ? AND type = ? AND args = ?
            ",
            script_id,
            exec_ty,
            args,
        )
        .execute(&*pool)
        .await?;

        sqlx::query!(
            "
            UPDATE events SET ignored = true
            WHERE script_id = ? AND type = ? AND main_event_id IN (
                SELECT id FROM events WHERE args = ? AND script_id = ?
            )
            ",
            script_id,
            done_ty,
            args,
            script_id,
        )
        .execute(&*pool)
        .await?;

        if number == 1 {
            log::info!("ignore last args");
            ignore_last_args(&*pool, script_id, exec_ty).await?;
            ignore_last_args(&*pool, script_id, done_ty).await?;
        }
        Ok(())
    }

    pub async fn tidy(&self, script_id: i64) -> Result<(), DBError> {
        let pool = self.pool.read().unwrap();
        let exec_ty = EventType::Exec.to_string();
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
                    ORDER BY time DESC
                    LIMIT 1
                  )
                FROM
                  (
                    SELECT distinct args
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
            exec_ty,
        )
        .execute(&*pool)
        .await?;

        Ok(())
    }
}
