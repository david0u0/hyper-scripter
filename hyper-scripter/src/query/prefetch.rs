use super::{ScriptQuery, ScriptQueryInner};
use crate::error::Result;
use crate::script_repo::{RepoEntry, ScriptRepo};

pub async fn prefetch<'b>(
    script_query: &ScriptQuery,
    script_repo: &'b mut ScriptRepo,
) -> Option<Result<Option<RepoEntry<'b>>>> {
    if script_repo.all_loaded() {
        return None;
    }

    log::debug!("嘗試預載入 `{:?}`", script_query);
    let bang = script_query.bang;
    match &script_query.inner {
        ScriptQueryInner::Exact(exact) => {
            // TODO: 支援 Extract 的預載入
        }
        ScriptQueryInner::Prev(exact) => {
            // TODO: 支援 Prev 的預載入
        }
        ScriptQueryInner::Fuzz(exact) => {
            if let Err(e) = script_repo.fetch_all().await {
                return Some(Err(e));
            }
        }
    }
    None
}
