use super::the_multifuzz_algo::{the_multifuzz_algo, MultiFuzzObj};
use super::{ListQuery, ScriptQuery, ScriptQueryInner};
use crate::color::Stylize;
use crate::config::{Config, PromptLevel};
use crate::error::{Error, Result};
use crate::fuzzy;
use crate::script_repo::{RepoEntry, ScriptRepo, Visibility};
use crate::util::{get_display_type, prompt};
use crate::Either;
use crate::SEP;
use fxhash::FxHashSet as HashSet;

fn compute_vis(bang: bool) -> Visibility {
    if bang {
        Visibility::All
    } else {
        Visibility::Normal
    }
}

pub async fn do_list_query<'a>(
    repo: &'a mut ScriptRepo,
    queries: impl IntoIterator<Item = ListQuery>,
) -> Result<Vec<RepoEntry<'a>>> {
    do_list_query_with_handler(repo, queries, &mut super::DefaultListQueryHandler).await
}

pub async fn do_list_query_with_handler<'a, I, H: super::ListQueryHandler<Item = I>>(
    repo: &'a mut ScriptRepo,
    queries: impl IntoIterator<Item = I>,
    handler: &mut H,
) -> Result<Vec<RepoEntry<'a>>> {
    let mut is_empty = true;
    let mut mem = HashSet::<i64>::default();
    let mut ret = vec![];
    let repo_ptr = repo as *mut ScriptRepo;
    for query in queries {
        is_empty = false;
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
        let query = handler.handle_item(query);
        match query {
            None => (),
            Some(ListQuery::Pattern(re, og, bang)) => {
                let mut is_empty = true;
                for script in repo.iter_mut(compute_vis(bang)) {
                    if re.is_match(&script.name.key()) {
                        is_empty = false;
                        insert!(script);
                    }
                }
                if is_empty {
                    return Err(Error::ScriptNotFound(og.to_owned()));
                }
            }
            Some(ListQuery::Query(query)) => {
                let script = match handler.handle_query(query, repo).await {
                    Ok(Some(entry)) => entry,
                    Ok(None) => continue,
                    Err(e) => return Err(e),
                };
                insert!(script);
            }
        }
    }
    if is_empty && H::should_return_all_on_empty() {
        return Ok(repo.iter_mut(Visibility::Normal).collect());
    }
    if ret.is_empty() && H::should_raise_dont_fuzz_on_empty() {
        log::debug!("列表查不到東西，卻又不是因為 pattern not match，想必是因為使用者取消了模糊搜");
        Err(Error::DontFuzz)
    } else {
        Ok(ret)
    }
}

impl<'a> MultiFuzzObj for RepoEntry<'a> {
    fn beats(&self, other: &Self) -> bool {
        self.last_time() > other.last_time()
    }
}

pub async fn do_script_query<'b>(
    script_query: &ScriptQuery,
    script_repo: &'b mut ScriptRepo,
    finding_filtered: bool,
    forbid_prompt: bool,
) -> Result<Option<RepoEntry<'b>>> {
    log::debug!("開始尋找 `{:?}`", script_query);
    let mut visibility = compute_vis(script_query.bang);
    if finding_filtered {
        visibility = visibility.invert();
    }
    match &script_query.inner {
        ScriptQueryInner::Prev(prev) => {
            assert!(!finding_filtered); // XXX 很難看的作法，應設法靜態檢查
            let latest = script_repo.latest_mut(prev.get(), visibility);
            log::trace!("找最新腳本");
            return if latest.is_some() {
                Ok(latest)
            } else {
                Err(Error::Empty)
            };
        }
        ScriptQueryInner::Exact(name) => Ok(script_repo.get_mut(name, visibility)),
        ScriptQueryInner::Fuzz(name) => {
            let level = if forbid_prompt {
                PromptLevel::Never
            } else {
                Config::get_prompt_level()
            };

            let iter = script_repo.iter_mut(visibility);
            let fuzz_res = fuzzy::fuzz(name, iter, SEP).await?;
            let mut is_low = false;
            let mut is_multi_fuzz = false;
            let entry = match fuzz_res {
                Some(fuzzy::High(entry)) => entry,
                Some(fuzzy::Low(entry)) => {
                    is_low = true;
                    entry
                }
                #[cfg(feature = "benching")]
                Some(fuzzy::Multi { ans, .. }) => {
                    is_multi_fuzz = true;
                    ans
                }
                #[cfg(not(feature = "benching"))]
                Some(fuzzy::Multi { ans, others, .. }) => {
                    is_multi_fuzz = true;
                    match handle_special_dot_anonymous(name, ans, others) {
                        Either::One(res) => res,
                        Either::Two((ans, others)) => the_multifuzz_algo(ans, others),
                    }
                }
                None => return Ok(None),
            };
            let need_prompt = {
                match level {
                    PromptLevel::Always => true,
                    PromptLevel::Never => false,
                    PromptLevel::Smart => is_low || is_multi_fuzz,
                    PromptLevel::OnMultiFuzz => is_multi_fuzz,
                }
            };
            if need_prompt {
                let ty = get_display_type(&entry.ty);
                let msg = format!("{}({})?", entry.name, ty.display());
                let yes = prompt(msg.stylize().color(ty.color()).bold(), true)?;
                if !yes {
                    return Err(Error::DontFuzz);
                }
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
    if let Some(info) = do_script_query(script_query, script_repo, false, false).await? {
        return Ok(info);
    }

    let script_repo = unsafe { &mut *ptr };
    #[cfg(not(feature = "benching"))]
    if !script_query.bang {
        let filtered = do_script_query(script_query, script_repo, true, true).await?;
        if let Some(filtered) = filtered {
            return Err(Error::ScriptIsFiltered(filtered.name.key().to_string()));
        }
    };

    Err(Error::ScriptNotFound(script_query.to_string()))
}

// 判斷特例：若收到的查詢是單一個 "."，且所有候選人皆為匿名，則不再考慮任何前綴問題
// 舉例：若有兩個腳本 .1 和 .10，用 "." 來查詢不該讓 .1 遮蔽掉 .10
// （但若是用 "1" 來查就該遮蔽）
fn handle_special_dot_anonymous<'a>(
    pattern: &str,
    ans: RepoEntry<'a>,
    others: Vec<RepoEntry<'a>>,
) -> Either<RepoEntry<'a>, (RepoEntry<'a>, Vec<RepoEntry<'a>>)> {
    if pattern != "." {
        return Either::Two((ans, others));
    }
    // TODO: maybe only check `ans` is enough?
    if !ans.name.is_anonymous() || others.iter().any(|e| !e.name.is_anonymous()) {
        return Either::Two((ans, others));
    }

    let res = std::iter::once(ans)
        .chain(others.into_iter())
        .max_by_key(|e| e.last_time())
        .unwrap();
    Either::One(res)
}
