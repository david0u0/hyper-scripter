use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::path::Path;

pub async fn get_pool(
    hyper_scripter_path: impl AsRef<Path>,
) -> Result<SqlitePool, sqlx::error::Error> {
    let file = hyper_scripter_path.as_ref().join(".script_history.db");
    let res = SqlitePool::connect_with(SqliteConnectOptions::new().filename(&file)).await;
    let pool = if res.is_err() {
        crate::migration::do_migrate(file).await?
    } else {
        res?
    };
    Ok(pool)
}
