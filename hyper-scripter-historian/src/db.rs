use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    SqlitePool,
};
use std::path::{Path, PathBuf};

pub fn get_file(dir_path: impl AsRef<Path>) -> PathBuf {
    dir_path.as_ref().join(".script_history.db")
}

pub async fn get_pool(dir_path: impl AsRef<Path>) -> Result<SqlitePool, sqlx::error::Error> {
    let file = get_file(dir_path);
    let opt = SqliteConnectOptions::new()
        .filename(&file)
        .journal_mode(SqliteJournalMode::Off);
    let res = SqlitePool::connect_with(opt).await;
    let pool = if res.is_err() {
        crate::migration::do_migrate(file).await?
    } else {
        res?
    };
    Ok(pool)
}
