use sqlx::migrate::MigrateError;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::path::Path;

pub async fn do_migrate(file: impl AsRef<Path>) -> Result<SqlitePool, MigrateError> {
    log::info!("進行資料庫遷移 {:?}！", file.as_ref());
    let pool = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(file)
            .create_if_missing(true),
    )
    .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
