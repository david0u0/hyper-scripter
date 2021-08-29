use super::{ListQuery, ScriptQuery, ScriptQueryInner};
use crate::config::{Config, PromptLevel};
use crate::error::{Error, Result};
use crate::fuzzy;
use crate::script::ScriptInfo;
use crate::script_repo::{RepoEntry, ScriptRepo};
use crate::util::get_display_type;
use fxhash::FxHashSet as HashSet;
use std::sync::Once;

const SEP: &str = "/";

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
            ListQuery::Pattern(re, og) => {
                let mut is_empty = true;
                for script in repo.iter_mut(false) {
                    if re.is_match(&script.name.key()) {
                        is_empty = false;
                        insert!(script);
                    }
                }
                if is_empty {
                    return Err(Error::ScriptNotFound(og.to_owned()));
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
    if ret.is_empty() {
        log::debug!("列表查不到東西，卻又不是因為 pattern not match，想必是因為使用者取消了模糊搜");
        Err(Error::DontFuzz)
    } else {
        Ok(ret)
    }
}

pub async fn do_script_query<'b>(
    script_query: &ScriptQuery,
    script_repo: &'b mut ScriptRepo,
    force_all: bool,
    forbid_prompt: bool,
) -> Result<Option<RepoEntry<'b>>> {
    log::debug!("開始尋找 `{:?}`", script_query);
    let all = script_query.bang || force_all;
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
            let level = if forbid_prompt {
                PromptLevel::Never
            } else {
                Config::get_prompt_level()
            };
            let fuzz_res = fuzzy::fuzz(name, script_repo.iter_mut(all), SEP).await?;
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
                #[cfg(feature = "benching")]
                Some(fuzzy::Multi { ans, .. }) => {
                    need_prompt = true;
                    ans
                }
                #[cfg(not(feature = "benching"))]
                Some(fuzzy::Multi { ans, others }) => {
                    need_prompt = true;
                    // NOTE: 從一堆分數相近者中選出最新的
                    // 但注意不要是「正解」的前綴，否則使用者可能永遠無法用模糊搜拿到名字比較短的候選者
                    // 例如 正解：ab 候選：ab1，且候選人較新
                    let prefix = ans.name.key();
                    let prefix = prefix.as_ref();
                    let true_ans = others
                        .into_iter()
                        .filter(|t| !fuzzy::is_prefix(prefix, t.name.key().as_ref(), SEP))
                        .max_by_key(|t| t.last_time());
                    match true_ans {
                        Some(t) if t.last_time() > ans.last_time() => t,
                        _ => ans,
                    }
                }
                None => return Ok(None),
            };
            if (need_prompt && !level.never()) || level.always() {
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
    // FIXME: 一旦 NLL 進化就修掉這段 unsafe
    let ptr = script_repo as *mut ScriptRepo;
    if let Some(info) = do_script_query(script_query, unsafe { &mut *ptr }, false, false).await? {
        return Ok(info);
    }

    #[cfg(not(feature = "benching"))]
    if !script_query.bang {
        let filtered = do_script_query(script_query, script_repo, true, true).await?;
        if let Some(filtered) = filtered {
            return Err(Error::ScriptIsFiltered(filtered.name.key().to_string()));
        }
    }

    Err(Error::ScriptNotFound(script_query.to_string()))
}

static CTRLC_HANDLE: Once = Once::new();
fn prompt_fuzz_acceptable(script: &ScriptInfo) -> Result {
    use colored::{Color, Colorize};
    use console::{Key, Term};

    let term = Term::stderr();

    let ty = get_display_type(&script.ty);
    let msg = format!(
        "{} [Y/N]",
        format!("{}({})?", script.name, ty.display())
            .color(ty.color())
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
