use super::{do_script_query_strict, ListQuery, ScriptQuery};
use crate::error::{Error, Result};
use crate::script::ScriptName;
use crate::script_repo::{RepoEntry, ScriptRepo, Visibility};

pub trait ListQueryHandler {
    type Item;
    async fn handle_query<'a, T: StableRepo>(
        &mut self,
        query: ScriptQuery,
        repo: &'a mut T,
    ) -> Result<Option<RepoEntry<'a>>>;
    fn handle_item(&mut self, item: Self::Item) -> Option<ListQuery>;
    fn should_raise_dont_fuzz_on_empty() -> bool;
    fn should_return_all_on_empty() -> bool;
}

pub struct DefaultListQueryHandler;

impl ListQueryHandler for DefaultListQueryHandler {
    type Item = ListQuery;
    async fn handle_query<'a, T: StableRepo>(
        &mut self,
        query: ScriptQuery,
        repo: &'a mut T,
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

/// A repo without insert & delete
pub trait StableRepo {
    fn iter_mut(&mut self, visibility: Visibility) -> impl Iterator<Item = RepoEntry<'_>>;
    fn latest_mut(&mut self, n: usize, visibility: Visibility) -> Option<RepoEntry<'_>>;
    fn get_mut(&mut self, name: &ScriptName, visibility: Visibility) -> Option<RepoEntry<'_>>;
}

impl StableRepo for ScriptRepo {
    fn iter_mut(&mut self, visibility: Visibility) -> impl Iterator<Item = RepoEntry<'_>> {
        self.iter_mut(visibility)
    }
    fn latest_mut(&mut self, n: usize, visibility: Visibility) -> Option<RepoEntry<'_>> {
        self.latest_mut(n, visibility)
    }
    fn get_mut(&mut self, name: &ScriptName, visibility: Visibility) -> Option<RepoEntry<'_>> {
        self.get_mut(name, visibility)
    }
}
