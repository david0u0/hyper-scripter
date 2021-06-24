use super::{ListQuery, ScriptQuery, ScriptQueryInner};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::fuzzy;
use crate::script::{IntoScriptName, ScriptInfo};
use crate::script_repo::{RepoEntry, ScriptRepo};
use fxhash::FxHashSet as HashSet;
use std::sync::Once;

pub async fn do_list_query<'a>(
    repo: &'a mut ScriptRepo,
    queries: &[ListQuery],
) -> Result<Vec<RepoEntry<'a>>> {
    if queries.is_empty() {
        return Ok(repo.iter_mut(false).collect());
    }
    let mut mem = HashSet::<i64>::default();
    let mut ret = vec![];
    let repo_ptr = repo as *mut ScriptRepo;
    for query in queries.iter() {
        macro_rules! insert {
            ($script:ident) => {
                if mem.contains(&$script.id) {
                    continue;
                }
                mem.insert($script.id);
                ret.push($script);
            };
        }
        // SAFETY: `mem` 已保證回傳的陣列不可能包含相同的資料
        let repo = unsafe { &mut *repo_ptr };
        match query {
            ListQuery::Pattern(re) => {
                for script in repo.iter_mut(false) {
                    if re.is_match(&script.name.key()) {
                        insert!(script);
                    }
                }
            }
            ListQuery::Query(query) => {
                let script = match do_script_query_strict(query, repo).await {
                    Err(Error::DontFuzz) => continue,
                    Ok(entry) => entry,
                    Err(e) => return Err(e),
                };
                insert!(script);
            }
        }
    }
    Ok(ret)
}

pub async fn do_script_query<'b>(
    script_query: &ScriptQuery,
    script_repo: &'b mut ScriptRepo,
) -> Result<Option<RepoEntry<'b>>> {
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
        ScriptQueryInner::Fuzz(name) => {
            let level = Config::get()?.prompt_level;
            let fuzz_res = fuzzy::fuzz(name, script_repo.iter_mut(all)).await?;
            let need_prompt: bool;
            let entry = match fuzz_res {
                Some(fuzzy::High(entry)) => {
                    need_prompt = false;
                    entry
                }
                Some(fuzzy::Low(entry)) => {
                    need_prompt = true;
                    entry
                }
                Some(fuzzy::Multi { ans, .. }) => {
                    need_prompt = true;
                    ans
                }
                None => return Ok(None),
            };
            if (need_prompt && !level.never()) | level.always() {
                prompt_fuzz_acceptable(&*entry)?;
            }
            Ok(Some(entry))
        }
    }
}
pub async fn do_script_query_strict<'b>(
    script_query: &ScriptQuery,
    script_repo: &'b mut ScriptRepo,
) -> Result<RepoEntry<'b>> {
    match do_script_query(script_query, script_repo).await {
        Err(e) => Err(e),
        Ok(None) => Err(Error::ScriptNotFound(
            script_query.clone().into_script_name()?.to_string(), // TODO: 簡單點？
        )),
        Ok(Some(info)) => Ok(info),
    }
}

static CTRLC_HANDLE: Once = Once::new();
fn prompt_fuzz_acceptable(script: &ScriptInfo) -> Result {
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
    CTRLC_HANDLE.call_once(|| {
        let term = term.clone();
        let term_hook_res = ctrlc::set_handler(move || {
            let _ = term.show_cursor();
            std::process::exit(1);
        });
        if term_hook_res.is_err() {
            log::warn!("設置 ctrl-c 回調失敗");
        }
    });

    let ok = loop {
        term.write_str(&msg)?;
        match term.read_key() {
            Ok(Key::Char('Y' | 'y') | Key::Enter) => break true,
            Ok(Key::Char('N' | 'n')) => break false,
            Ok(Key::Char(ch)) => term.write_line(&format!(" Unknown key '{}'", ch))?,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::Interrupted {
                    term.show_cursor()?;
                    std::process::exit(1);
                } else {
                    return Err(e.into());
                }
            }
            _ => term.write_line(&format!(" Unknown key"))?,
        }
    };
    term.show_cursor()?;
    if ok {
        term.write_line(&" Y".color(Color::Green).to_string())?;
        Ok(())
    } else {
        term.write_line(&" N".color(Color::Red).to_string())?;
        Err(Error::DontFuzz)
    }
}
