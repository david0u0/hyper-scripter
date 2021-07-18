use crate::error::Result;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    SqlitePool,
};

pub async fn get_pool() -> Result<(SqlitePool, bool)> {
    let file = crate::path::get_home().join(".script_info.db");
    let opt = SqliteConnectOptions::new()
        .filename(&file)
        .journal_mode(SqliteJournalMode::Off);
    let res = SqlitePool::connect_with(opt).await;
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
