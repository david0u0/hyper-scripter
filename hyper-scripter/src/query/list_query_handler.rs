use super::{do_script_query_strict, ListQuery, ScriptQuery};
use crate::error::{Error, Result};
use crate::script_repo::{RepoEntry, ScriptRepo};

// SAFETY: 此特徵的實作應保證永遠不改動 repo 本身（只可回傳其可變參照），否則先前存下的參照可能會無效化
pub unsafe trait ListQueryHandler {
    type Item;
    async fn handle_query<'a>(
        &mut self,
        query: ScriptQuery,
        repo: &'a mut ScriptRepo,
    ) -> Result<Option<RepoEntry<'a>>>;
    fn handle_item(&mut self, item: Self::Item) -> Option<ListQuery>;
    fn should_raise_dont_fuzz_on_empty() -> bool;
    fn should_return_all_on_empty() -> bool;
}

pub struct DefaultListQueryHandler;

// SAFETY: 實作永不改動 repo 本身
unsafe impl ListQueryHandler for DefaultListQueryHandler {
    type Item = ListQuery;
    async fn handle_query<'a>(
        &mut self,
        query: ScriptQuery,
        repo: &'a mut ScriptRepo,
    ) -> Result<Option<RepoEntry<'a>>> {
        match do_script_query_strict(&query, repo).await {
            Ok(script) => Ok(Some(script)),
            Err(Error::DontFuzz) => Ok(None),
            Err(err) => Err(err),
        }
    }
    fn handle_item(&mut self, item: Self::Item) -> Option<ListQuery> {
        Some(item)
    }
    fn should_raise_dont_fuzz_on_empty() -> bool {
        true
    }
    fn should_return_all_on_empty() -> bool {
        true
    }
}
