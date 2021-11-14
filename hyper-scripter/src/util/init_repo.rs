use super::main_util;
use crate::args::RootArgs;
use crate::config::Config;
use crate::error::{Contextable, Error, Result};
use crate::path;
use crate::script_repo::{RecentFilter, ScriptRepo};
use fxhash::FxHashSet as HashSet;
use hyper_scripter_historian::Historian;

/// 即使 `need_journal=false` 也可能使用 journal，具體條件同 `crate::db::get_pool`
pub async fn init_repo(args: RootArgs, mut need_journal: bool) -> Result<ScriptRepo> {
    let RootArgs {
        no_trace,
        humble,
        archaeology,
        filter,
        toggle,
        recent,
        timeless,
        ..
    } = args;

    let conf = Config::get();
    let (pool, init) = crate::db::get_pool(&mut need_journal).await?;

    let recent = if timeless {
        None
    } else {
        recent.or(conf.recent).map(|recent| RecentFilter {
            recent,
            archaeology,
        })
    };

    let historian = Historian::new(path::get_home().to_owned()).await?;
    let mut repo = ScriptRepo::new(pool, recent, historian, need_journal)
        .await
        .context("讀取歷史記錄失敗")?;
    if no_trace {
        repo.no_trace();
    } else if humble {
        repo.humble();
    }

    if init {
        log::info!("初次使用，載入好用工具和預執行腳本");
        main_util::load_utils(&mut repo).await?;
        main_util::prepare_pre_run()?;
        main_util::load_templates()?;
    }

    // TODO: 測試 toggle 功能，以及名字不存在的錯誤
    let mut toggle: HashSet<_> = toggle.into_iter().collect();
    let mut tag_group = conf.get_tag_filter_group(&mut toggle);
    if let Some(name) = toggle.into_iter().next() {
        return Err(Error::TagFilterNotFound(name));
    }
    for filter in filter.into_iter() {
        tag_group.push(filter);
    }
    repo.filter_by_tag(&tag_group);

    Ok(repo)
}
