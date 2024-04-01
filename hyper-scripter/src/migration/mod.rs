use sqlx::migrate::MigrateError;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::path::Path;

pub async fn do_migrate_with_pre_sql(
    file: impl AsRef<Path>,
    pre_sql: Option<&str>,
) -> Result<SqlitePool, MigrateError> {
    log::info!("進行資料庫遷移 {:?}！", file.as_ref());
    let pool = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(file)
            .create_if_missing(true),
    )
    .await?;

    if let Some(pre_sql) = pre_sql {
        log::info!("Apply db pre script {}！", pre_sql);
        sqlx::query(pre_sql).execute(&pool).await?;
    }

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
