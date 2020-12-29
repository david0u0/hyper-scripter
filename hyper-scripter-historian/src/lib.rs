#[macro_use]
extern crate derive_more;

use chrono::NaiveDateTime;
use sqlx::{error::Error as DBError, Pool, Sqlite, SqlitePool};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

mod db;
mod event;
pub mod migration;
pub use event::*;

#[derive(Debug, Clone)]
pub struct Historian {
    pool: Arc<Mutex<SqlitePool>>,
    path: PathBuf,
}

async fn raw_record_event(
    pool: &Pool<Sqlite>,
    script_id: i64,
    ty: &str,
    cmd: &str,
    content: Option<&str>,
) -> Result<(), DBError> {
    sqlx::query!(
        "INSERT INTO events (script_id, type, cmd, content) VALUES(?, ?, ?, ?)",
        script_id,
        ty,
        cmd,
        content
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
    content: Option<&str>,
) -> Result<(), DBError> {
    sqlx::query!(
        "INSERT OR REPLACE INTO last_events (script_id, type, cmd, content) VALUES(?, ?, ?, ?)",
        script_id,
        ty,
        cmd,
        content
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
        content: Option<&str>,
    ) -> Result<(), DBError> {
        let pool = &*self.pool.lock().unwrap();
        let res = raw_record_last(pool, script_id, ty, cmd, content).await;
        if res.is_err() {
            log::warn!("資料庫錯誤 {:?}，再試最後一次！", res);
            let new_pool = db::get_pool(&self.path).await?;
            *self.pool.lock().unwrap() = new_pool;
            raw_record_last(pool, script_id, ty, cmd, content).await?;
            raw_record_event(pool, script_id, ty, cmd, content).await?;
            return Ok(());
        }

        raw_record_event(pool, script_id, ty, cmd, content).await
    }
    pub async fn new(path: impl AsRef<Path>) -> Result<Self, DBError> {
        let path = path.as_ref().to_owned();
        db::get_pool(&path).await.map(|pool| Historian {
            pool: Arc::new(Mutex::new(pool)),
            path,
        })
    }

    pub async fn record(&self, event: &Event<'_>) -> Result<(), DBError> {
        log::debug!("記錄事件 {:?}", event);
        let cmd = std::env::args().collect::<Vec<_>>().join(" ");
        let ty = event.data.get_type().to_string();
        match &event.data {
            EventData::Miss => {
                self.raw_record(event.script_id, &ty, &cmd, None).await?;
            }
            EventData::Read => {
                self.raw_record(event.script_id, &ty, &cmd, None).await?;
            }
            EventData::Exec(content) => {
                let mut content = Some(*content);
                let last_event = sqlx::query!(
                    "SELECT content FROM events
                WHERE type = ? AND script_id = ? AND NOT content IS NULL
                ORDER BY time DESC LIMIT 1",
                    ty,
                    event.script_id
                )
                .fetch_optional(&*self.pool.lock().unwrap())
                .await?;
                if let Some(last_event) = last_event {
                    if last_event.content.as_ref().map(|s| s.as_str()) == content {
                        log::debug!("上次執行內容相同，不重覆記錄");
                        content = None;
                    }
                }
                self.raw_record(event.script_id, &ty, &cmd, content).await?;
            }
            EventData::ExecDone(code) => {
                let code = code.to_string();
                self.raw_record(event.script_id, &ty, &cmd, Some(&code))
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn last_time_of(&self, ty: EventType) -> Result<Vec<(i64, NaiveDateTime)>, DBError> {
        let ty = ty.to_string();
        let times = sqlx::query!(
            "SELECT script_id, time as time FROM last_events WHERE type = ? ORDER BY script_id",
            ty
        )
        .fetch_all(&*self.pool.lock().unwrap())
        .await?;
        Ok(times.into_iter().map(|d| (d.script_id, d.time)).collect())
    }
}
