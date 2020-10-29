use crate::error::Result;
use crate::fuzzy::FuzzKey;
use crate::script::ScriptInfo;
use async_trait::async_trait;
use std::collections::hash_map::IterMut as HashMapIter;

#[async_trait]
pub trait Environment {
    async fn handle_change(&self, info: &ScriptInfo) -> Result;
}

pub struct Iter<'b, ENV: Environment> {
    pub(super) iter: HashMapIter<'b, String, ScriptInfo>,
    pub(super) env: &'b ENV,
}
#[derive(Deref)]
pub struct RepoEntry<'b, ENV: Environment> {
    #[deref]
    pub(super) info: &'b mut ScriptInfo,
    pub(super) env: &'b ENV,
}

impl<'a, 'b, ENV: Environment> RepoEntry<'b, ENV> {
    pub async fn update<F: FnOnce(&mut ScriptInfo)>(&mut self, handler: F) -> Result {
        handler(self.info);
        self.env.handle_change(self.info).await
    }
    pub fn into_inner(self) -> &'b mut ScriptInfo {
        self.info
    }
}
impl<'a, 'b, ENV: Environment> Iterator for Iter<'b, ENV> {
    type Item = RepoEntry<'b, ENV>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_, info)| RepoEntry {
            info,
            env: self.env,
        })
    }
}
impl<'a, 'b, ENV: Environment> FuzzKey for RepoEntry<'b, ENV> {
    fn fuzz_key<'c>(&'c self) -> std::borrow::Cow<'c, str> {
        self.info.fuzz_key()
    }
}
