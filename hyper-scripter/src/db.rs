use crate::error::Result;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    SqlitePool,
};

/// 有可能改變 need_journal 的值。若為初始化，不論如何都使用 journal
pub async fn get_pool(need_journal: &mut bool) -> Result<(SqlitePool, bool)> {
    let file = crate::path::get_home().join(".script_info.db");
    let mut opt = SqliteConnectOptions::new().filename(&file);
    if !*need_journal {
        opt = opt.journal_mode(SqliteJournalMode::Off);
    }
    let res = SqlitePool::connect_with(opt).await;
    let init: bool;
    let pool = match res {
        Err(_) => {
            init = true;
            *need_journal = true;
            crate::migration::do_migrate(file).await?
        }
        Ok(pool) => {
            init = false;
            pool
        }
    };
    Ok((pool, init))
}
