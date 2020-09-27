#[macro_use]
extern crate derive_more;

use chrono::NaiveDateTime;
use sqlx::{error::Error as DBError, SqlitePool};
use std::path::Path;

mod db;
mod event;
pub mod migration;
pub use event::*;

#[derive(Debug, Clone)]
pub struct Historian {
    pool: SqlitePool,
}

impl Historian {
    pub async fn new(hyper_scripter_path: impl AsRef<Path>) -> Result<Self, DBError> {
        db::get_pool(hyper_scripter_path)
            .await
            .map(|pool| Historian { pool })
    }

    pub async fn record(&self, event: Event<'_>) -> Result<(), DBError> {
        log::debug!("記錄事件 {:?}", event);
        let cmd = std::env::args().collect::<Vec<_>>().join(" ");
        let ty = event.data.get_type().to_string();
        match &event.data {
            EventData::Read => {
                sqlx::query!(
                    "INSERT INTO events (script_id, type, cmd) VALUES(?, ?, ?)",
                    event.script_id,
                    ty,
                    cmd
                )
                .execute(&self.pool)
                .await?;
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
                .fetch_optional(&self.pool)
                .await?;
                if let Some(last_event) = last_event {
                    if last_event.content.as_ref().map(|s| s.as_str()) == content {
                        log::debug!("上次執行內容相同，不重覆記錄");
                        content = None;
                    }
                }
                sqlx::query!(
                    "INSERT INTO events (script_id, type, cmd, content) VALUES(?, ?, ?, ?)",
                    event.script_id,
                    ty,
                    cmd,
                    content,
                )
                .execute(&self.pool)
                .await?;
            }
            EventData::ExecDone(code) => {
                let code = code.to_string();
                sqlx::query!(
                    "INSERT INTO events (script_id, type, cmd, content) VALUES(?, ?, ?, ?)",
                    event.script_id,
                    ty,
                    cmd,
                    code
                )
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn last_time_of(&self, ty: EventType) -> Result<Vec<(i64, NaiveDateTime)>, DBError> {
        let ty = ty.to_string();
        let records = sqlx::query_as(
            "
        SELECT e.script_id, MAX(e.time) as time FROM events e
        WHERE type = ?
        GROUP BY e.script_id ORDER BY script_id 
        ",
        )
        .bind(ty)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
        // Ok(records.into_iter().map(|r| (r.script_id, r.time)).collect())
    }
}
