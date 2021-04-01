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

async fn ignore_last_args(pool: &Pool<Sqlite>, script_id: i64) -> Result<(), DBError> {
    let exec_ty = EventType::Exec.to_string();
    sqlx::query!(
        "DELETE FROM last_events WHERE script_id = ? AND type = ?",
        script_id,
        exec_ty,
    )
    .execute(pool)
    .await?;

    let last_event = sqlx::query!(
        "
        SELECT * FROM events
        WHERE type = ? AND script_id = ? AND NOT ignored
        ORDER BY time DESC LIMIT 1
        ",
        exec_ty,
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
            exec_ty,
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
async fn raw_record_event(
    pool: &Pool<Sqlite>,
    script_id: i64,
    ty: &str,
    cmd: &str,
    args: Option<&str>,
    content: Option<&str>,
    time: NaiveDateTime,
) -> Result<(), DBError> {
    sqlx::query!(
        "INSERT INTO events (script_id, type, cmd, args, content, time) VALUES(?, ?, ?, ?, ?, ?)",
        script_id,
        ty,
        cmd,
        args,
        content,
        time
    )
    .execute(pool)
    .await?;
    Ok(())
}
async fn raw_record_last(
    pool: &Pool<Sqlite>,
    script_id: i64,
    ty: &str,
    cmd: &str,
    args: Option<&str>,
    content: Option<&str>,
    time: NaiveDateTime,
) -> Result<(), DBError> {
    sqlx::query!(
        "INSERT OR REPLACE INTO last_events (script_id, type, cmd, args, content, time) VALUES(?, ?, ?, ?, ?, ?)",
        script_id,
        ty,
        cmd,
        args,
        content,
        time
    )
    .execute(pool)
    .await?;
    Ok(())
}
impl Historian {
    async fn raw_record(
        &self,
        script_id: i64,
        ty: &str,
        cmd: &str,
        args: Option<&str>,
        content: Option<&str>,
        time: NaiveDateTime,
    ) -> Result<(), DBError> {
        let pool = &mut *self.pool.write().unwrap();
        let res = raw_record_last(pool, script_id, ty, cmd, args, content, time).await;
        if res.is_err() {
            log::warn!("資料庫錯誤 {:?}，再試最後一次！", res);
            *pool = db::get_pool(&self.path).await?;
            raw_record_last(pool, script_id, ty, cmd, args, content, time).await?;
            raw_record_event(pool, script_id, ty, cmd, args, content, time).await?;
            return Ok(());
        }

        raw_record_event(pool, script_id, ty, cmd, args, content, time).await
    }
    pub async fn new(path: impl AsRef<Path>) -> Result<Self, DBError> {
        let path = path.as_ref().to_owned();
        db::get_pool(&path).await.map(|pool| Historian {
            pool: Arc::new(RwLock::new(pool)),
            path,
        })
    }

    pub async fn record(&self, event: &Event<'_>) -> Result<(), DBError> {
        log::debug!("記錄事件 {:?}", event);
        let cmd = std::env::args().collect::<Vec<_>>().join(" ");
        let ty = event.data.get_type().to_string();
        let time = event.time;
        match &event.data {
            EventData::Miss => {
                self.raw_record(event.script_id, &ty, &cmd, None, None, time)
                    .await?;
            }
            EventData::Read => {
                self.raw_record(event.script_id, &ty, &cmd, None, None, time)
                    .await?;
            }
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
                    if last_event.content.as_ref().map(|s| s.as_str()) == content {
                        log::debug!("上次執行內容相同，不重複記錄");
                        content = None;
                    }
                }
                self.raw_record(event.script_id, &ty, &cmd, Some(args), content, time)
                    .await?;
            }
            EventData::ExecDone(code) => {
                let code = code.to_string();
                self.raw_record(event.script_id, &ty, &cmd, None, Some(&code), time)
                    .await?;
            }
        }
        Ok(())
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

    pub async fn ignore_args(&self, script_id: i64, number: u32) -> Result<(), DBError> {
        let ty = EventType::Exec.to_string();
        let offset = number as i64 - 1;
        let pool = self.pool.read().unwrap();
        sqlx::query!(
            "
            WITH args_table AS (
                SELECT args, max(time) as time FROM events
                WHERE type = ? AND script_id = ? AND NOT ignored
                GROUP BY args
                ORDER BY time DESC LIMIT 1 OFFSET ?
            ) 
            UPDATE events SET ignored = true
            WHERE script_id = ?
            AND args = (SELECT args FROM args_table)
            ",
            ty,
            script_id,
            offset,
            script_id,
        )
        .execute(&*pool)
        .await?;
        if number == 1 {
            log::info!("ignore last args");
            ignore_last_args(&*pool, script_id).await?;
        }
        Ok(())
    }
}
