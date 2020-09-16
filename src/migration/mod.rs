use sqlx::migrate::MigrateError;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::path::Path;

pub async fn do_migrate(dir: impl AsRef<Path>) -> Result<(), MigrateError> {
    let path = dir.as_ref().join("script_info.db");
    let pool = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true),
    )
    .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(())
}
