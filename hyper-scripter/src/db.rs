use crate::error::Result;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};

pub async fn get_pool() -> Result<SqlitePool> {
    let file = crate::path::get_path().join(".script_info.db");
    let res = SqlitePool::connect_with(SqliteConnectOptions::new().filename(&file)).await;
    let pool = if res.is_err() {
        crate::migration::do_migrate(file).await?
    } else {
        res?
    };
    Ok(pool)
}
