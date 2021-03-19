use super::{ListQuery, ScriptQuery, ScriptQueryInner};
use crate::error::{Error, Result};
use crate::fuzzy;
use crate::script::{IntoScriptName, ScriptInfo};
use crate::script_repo::{ScriptRepo, ScriptRepoEntry};
use crate::script_time::ScriptTime;
use fxhash::FxHashSet as HashSet;

pub async fn do_list_query<'a>(
    repo: &'a mut ScriptRepo,
    queries: &[ListQuery],
) -> Result<Vec<ScriptRepoEntry<'a>>> {
    if queries.len() == 0 {
        return Ok(repo.iter_mut(false).collect());
    }
    let mut mem = HashSet::<i64>::default();
    let mut ret = vec![];
    let repo_ptr = repo as *mut ScriptRepo;
    for query in queries.iter() {
        // SAFETY: `mem` 已保證回傳的陣列不可能包含相同的資料
        let repo = unsafe { &mut *repo_ptr };
        match query {
            ListQuery::Pattern(re) => {
                for script in repo.iter_mut(false) {
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
                let script = match do_script_query_strict(query, repo).await {
                    Err(Error::DontFuzz) => continue,
                    Ok(entry) => entry,
                    Err(e) => return Err(e),
                };
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

pub async fn do_script_query<'b>(
    script_query: &ScriptQuery,
    script_repo: &'b mut ScriptRepo,
) -> Result<Option<ScriptRepoEntry<'b>>> {
    log::debug!("開始尋找 `{:?}`", script_query);
    let all = script_query.bang;
    match &script_query.inner {
        ScriptQueryInner::Prev(prev) => {
            let latest = script_repo.latest_mut(*prev, all);
            log::trace!("找最新腳本");
            return if latest.is_some() {
                Ok(latest)
            } else {
                Err(Error::Empty)
            };
        }
        ScriptQueryInner::Exact(name) => Ok(script_repo.get_mut(name, all)),
        ScriptQueryInner::Fuzz(name) => match fuzzy::fuzz(name, script_repo.iter_mut(all)).await? {
            Some((entry, fuzzy::High)) => Ok(Some(entry)),
            Some((entry, fuzzy::Low)) => {
                if prompt_fuzz_acceptable(&*entry)? {
                    Ok(Some(entry))
                } else {
                    Err(Error::DontFuzz)
                }
            }
            _ => Ok(None),
        },
    }
}
pub async fn do_script_query_strict<'b>(
    script_query: &ScriptQuery,
    script_repo: &'b mut ScriptRepo,
) -> Result<ScriptRepoEntry<'b>> {
    match do_script_query(script_query, script_repo).await {
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
    match do_script_query(script_query, script_repo).await {
        Err(e) => Err(e),
        Ok(Some(info)) => Ok(info),
        Ok(None) => {
            let repo = unsafe { &mut *repo_mut };
            let info = match &script_query.inner {
                ScriptQueryInner::Exact(name) => repo.get_hidden_mut(name),
                ScriptQueryInner::Fuzz(name) => {
                    match fuzzy::fuzz(name, repo.iter_hidden_mut()).await {
                        Ok(Some((info, _))) => Some(info),
                        _ => None,
                    }
                }
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

#[cfg(not(feature = "no-prompt"))]
fn prompt_fuzz_acceptable(script: &ScriptInfo) -> Result<bool> {
    use crate::config::Config;
    use colored::{Color, Colorize};
    use console::{Key, Term};

    let term = Term::stderr();

    let color = Config::get()?.get_color(&script.ty)?;
    let msg = format!(
        "{} [Y/N]",
        format!("{}({})?", script.name, script.ty)
            .color(color)
            .bold(),
    );
    term.hide_cursor()?;
    let ok = loop {
        term.write_str(&msg)?;
        match term.read_key()? {
            Key::Char('Y') => break true,
            Key::Char('y') => break true,
            Key::Char('N') => break false,
            Key::Char('n') => break false,
            Key::Char(ch) => term.write_line(&format!(" Unknown key '{}'", ch))?,
            _ => break true,
        }
    };
    term.show_cursor()?;
    if ok {
        term.write_line(&" Y".color(Color::Green).to_string())?;
    } else {
        term.write_line(&" N".color(Color::Red).to_string())?;
    }
    Ok(ok)
}

#[cfg(feature = "no-prompt")]
fn prompt_fuzz_acceptable(_: &ScriptInfo) -> Result<bool> {
    Ok(true)
}
