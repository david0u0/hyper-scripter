use super::main_util;
use crate::args::RootArgs;
use crate::config::Config;
use crate::error::{Contextable, Error, Result};
use crate::path;
use crate::script_repo::{DBEnv, RecentFilter, ScriptRepo};
use futures::try_join;
use fxhash::FxHashSet as HashSet;
use hyper_scripter_historian::Historian;

/// 即使 `need_journal=false` 也可能使用 journal，具體條件同 `crate::db::get_pool`
pub async fn init_env(mut need_journal: bool) -> Result<(DBEnv, bool)> {
    async fn init_historian() -> Result<Historian> {
        let h = Historian::new(path::get_home().to_owned()).await?;
        Ok(h)
    }
    let ((pool, init), historian) =
        try_join!(crate::db::get_pool(&mut need_journal), init_historian())?;
    Ok((DBEnv::new(pool, historian, need_journal), init))
}

/// 即使 `need_journal=false` 也可能使用 journal，具體條件同 `crate::db::get_pool`
pub async fn init_repo(args: RootArgs, need_journal: bool) -> Result<ScriptRepo> {
    let RootArgs {
        no_trace,
        humble,
        archaeology,
        select,
        toggle,
        recent,
        timeless,
        ..
    } = args;

    let conf = Config::get();

    let recent = if timeless {
        None
    } else {
        recent.or(conf.recent).map(|recent| RecentFilter {
            recent,
            archaeology,
        })
    };

    // TODO: 測試 toggle 功能，以及名字不存在的錯誤
    let tag_group = {
        let mut toggle: HashSet<_> = toggle.into_iter().collect();
        let mut tag_group = conf.get_tag_selector_group(&mut toggle);
        if let Some(name) = toggle.into_iter().next() {
            return Err(Error::TagSelectorNotFound(name));
        }
        for select in select.into_iter() {
            tag_group.push(select);
        }
        tag_group
    };

    let (env, init) = init_env(need_journal).await?;
    let mut repo = ScriptRepo::new(recent, env, &tag_group)
        .await
        .context("載入腳本倉庫失敗")?;
    if no_trace {
        repo.no_trace();
    } else if humble {
        repo.humble();
    }

    if init {
        log::info!("初次使用，載入好用工具和預執行腳本");
        main_util::load_utils(&mut repo).await?;
        main_util::prepare_pre_run(None)?;
        main_util::load_templates()?;
    }

    Ok(repo)
}
