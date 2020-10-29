use super::{ListQuery, ScriptQuery};
use crate::error::{Error, Result};
use crate::fuzzy;
use crate::script::IntoScriptName;
use crate::script_repo::{ScriptRepo, ScriptRepoEntry};
use crate::script_time::ScriptTime;
use std::collections::HashSet;

pub fn do_list_query<'a>(
    repo: &'a mut ScriptRepo,
    queries: &[ListQuery],
) -> Result<Vec<ScriptRepoEntry<'a>>> {
    if queries.len() == 0 {
        return Ok(repo.iter_mut().collect());
    }
    let mut mem = HashSet::<i64>::new();
    let mut ret = vec![];
    let repo_ptr = repo as *mut ScriptRepo;
    for query in queries.iter() {
        // SAFETY: `mem` 已保證回傳的陣列不可能包含相同的資料
        let repo = unsafe { &mut *repo_ptr };
        match query {
            ListQuery::Pattern(re) => {
                for script in repo.iter_mut() {
                    if re.is_match(&script.name.key()) {
                        if mem.contains(&script.id) {
                            continue;
                        }
                        mem.insert(script.id);
                        ret.push(script);
                    }
                }
            }
            ListQuery::Query(query) => {
                let script = do_script_query_strict(query, repo)?;
                if mem.contains(&script.id) {
                    continue;
                }
                mem.insert(script.id);
                ret.push(script);
            }
        }
    }
    Ok(ret)
}

pub fn do_script_query<'b>(
    script_query: &ScriptQuery,
    script_repo: &'b mut ScriptRepo,
) -> Result<Option<ScriptRepoEntry<'b>>> {
    log::debug!("開始尋找 `{:?}`", script_query);
    match script_query {
        ScriptQuery::Prev(prev) => {
            let latest = script_repo.latest_mut(*prev);
            log::trace!("找最新腳本");
            return if latest.is_some() {
                Ok(latest)
            } else {
                Err(Error::Empty)
            };
        }
        ScriptQuery::Exact(name) => Ok(script_repo.get_mut(name)),
        ScriptQuery::Fuzz(name) => fuzzy::fuzz(name, script_repo.iter_mut()),
    }
}
pub fn do_script_query_strict<'b>(
    script_query: &ScriptQuery,
    script_repo: &'b mut ScriptRepo,
) -> Result<ScriptRepoEntry<'b>> {
    match do_script_query(script_query, script_repo) {
        Err(e) => Err(e),
        Ok(None) => Err(Error::ScriptNotFound(
            script_query.clone().into_script_name()?.to_string(), // TODO: 簡單點？
        )),
        Ok(Some(info)) => Ok(info),
    }
}

pub async fn do_script_query_strict_with_missing<'b>(
    script_query: &ScriptQuery,
    script_repo: &'b mut ScriptRepo,
) -> Result<ScriptRepoEntry<'b>> {
    let repo_mut = script_repo as *mut ScriptRepo;
    // FIXME: 一旦 NLL 進化就修掉這段 unsafe
    match do_script_query(script_query, script_repo) {
        Err(e) => Err(e),
        Ok(Some(info)) => Ok(info),
        Ok(None) => {
            let repo = unsafe { &mut *repo_mut };
            let info = match script_query {
                ScriptQuery::Exact(name) => repo.get_hidden_mut(name),
                ScriptQuery::Fuzz(name) => match fuzzy::fuzz(name, repo.iter_hidden_mut()) {
                    Ok(info) => info,
                    _ => None,
                },
                _ => None,
            };
            if let Some(mut info) = info {
                info.update(|info| {
                    info.miss_time = Some(ScriptTime::now(()));
                })
                .await?;
            }
            Err(Error::ScriptNotFound(
                script_query.clone().into_script_name()?.to_string(),
            ))
        }
    }
}
