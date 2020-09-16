use sqlx::migrate::{Migrate, MigrateError, Migrator};
use sqlx::{sqlite::SqliteConnectOptions, Connection, SqliteConnection};
use std::path::Path;

pub async fn do_migrate(dir: impl AsRef<Path>) -> Result<(), MigrateError> {
    let path = dir.as_ref().join("script_info.db");
    let mut conn = SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true),
    )
    .await?;

    let migrator = Migrator::new(std::path::Path::new("./migrations")).await?;
    conn.ensure_migrations_table().await?;

    let (version, dirty) = conn.version().await?.unwrap_or((0, false));
    if dirty {
        return Err(MigrateError::Dirty(version));
    }

    for migration in migrator.iter() {
        if migration.version > version {
            let elapsed = conn.apply(migration).await?;
            log::info!(
                "{}/遷移 {} ({:?})",
                migration.version,
                migration.description,
                elapsed,
            );
        } else {
            conn.validate(migration).await?;
        }
    }
    Ok(())
}
