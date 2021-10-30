use crate::error::Result;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
    SqlitePool,
};

/// 有可能改變 need_journal 的值。
/// 若為初始化，或資料庫已被 journal 鎖住，則不論如何都使用 journal
pub async fn get_pool(need_journal: &mut bool) -> Result<(SqlitePool, bool)> {
    let file = crate::path::get_home().join(".script_info.db");
    if !file.exists() {
        *need_journal = true;
        let pool = crate::migration::do_migrate(file).await?;
        return Ok((pool, true));
    }

    let mut opt = SqliteConnectOptions::new().filename(&file);
    if !*need_journal {
        opt = opt.journal_mode(SqliteJournalMode::Off);
    }
    let res = SqlitePool::connect_with(opt).await;
    let pool = match res {
        Err(err) => {
            // 通常是有其它程序用 journal mode 鎖住資料庫，例如正在編輯另一個腳本
            log::warn!("資料庫錯誤 {}，嘗試用 journal 再開一次", err);
            *need_journal = true;
            let opt = SqliteConnectOptions::new().filename(&file);
            SqlitePool::connect_with(opt).await?
        }
        Ok(pool) => pool,
    };
    Ok((pool, false))
}
