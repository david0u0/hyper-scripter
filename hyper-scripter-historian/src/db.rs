use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    SqlitePool,
};
use std::path::Path;

pub async fn get_pool(file: impl AsRef<Path>) -> Result<SqlitePool, sqlx::error::Error> {
    let opt = SqliteConnectOptions::new()
        .filename(file.as_ref())
        .journal_mode(SqliteJournalMode::Off);
    let res = SqlitePool::connect_with(opt).await;
    let pool = if res.is_err() {
        crate::migration::do_migrate(file).await?
    } else {
        res?
    };
    Ok(pool)
}
