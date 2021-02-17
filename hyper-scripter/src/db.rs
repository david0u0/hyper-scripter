use crate::error::Result;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};

pub async fn get_pool() -> Result<(SqlitePool, bool)> {
    let file = crate::path::get_home().join(".script_info.db");
    let res = SqlitePool::connect_with(SqliteConnectOptions::new().filename(&file)).await;
    let init: bool;
    let pool = if res.is_err() {
        init = true;
        crate::migration::do_migrate(file).await?
    } else {
        init = false;
        res?
    };
    Ok((pool, init))
}
