mod event;
pub use event::*;

use chrono::NaiveDateTime;
use sqlx::error::Error as DBError;
use sqlx::SqlitePool;

pub async fn record(event: Event, pool: &SqlitePool) -> Result<(), DBError> {
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
            .execute(pool)
            .await?;
        }
        EventData::Exec(content) => {
            // TODO: 如果跟上次內容相同就別存了
            sqlx::query!(
                "INSERT INTO events (script_id, type, cmd, content) VALUES(?, ?, ?, ?)",
                event.script_id,
                ty,
                cmd,
                content
            )
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

pub async fn last_time_of(
    ty: EventType,
    pool: &SqlitePool,
) -> Result<Vec<(i64, NaiveDateTime)>, DBError> {
    let ty = ty.to_string();
    let records = sqlx::query_as(
        "
        SELECT e.script_id, MAX(e.time) as time FROM events e
        WHERE type = ?
        GROUP BY e.script_id ORDER BY script_id 
        ",
    )
    .bind(ty)
    .fetch_all(pool)
    .await?;
    Ok(records)
    // Ok(records.into_iter().map(|r| (r.script_id, r.time)).collect())
}
