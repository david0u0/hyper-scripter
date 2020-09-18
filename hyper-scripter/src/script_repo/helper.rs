use crate::error::Result;
use crate::fuzzy::FuzzKey;
use crate::script::ScriptInfo;
use async_trait::async_trait;
use std::collections::hash_map::IterMut as HashMapIter;
use std::ops::Deref;

#[async_trait]
pub trait Environment {
    async fn handle_change<'a>(&self, info: &ScriptInfo<'a>) -> Result;
}

pub struct Iter<'a, 'b, ENV: Environment> {
    pub(super) iter: HashMapIter<'b, String, ScriptInfo<'a>>,
    pub(super) env: &'b ENV,
}
pub struct RepoEntry<'a, 'b, ENV: Environment> {
    pub info: &'b mut ScriptInfo<'a>,
    pub(super) env: &'b ENV,
}

impl<'a, 'b, ENV: Environment> Deref for RepoEntry<'a, 'b, ENV> {
    type Target = ScriptInfo<'a>;
    fn deref(&self) -> &Self::Target {
        self.info
    }
}
impl<'a, 'b, ENV: Environment> RepoEntry<'a, 'b, ENV> {
    pub async fn update<F: FnOnce(&mut ScriptInfo<'a>)>(&mut self, handler: F) -> Result {
        handler(self.info);
        self.env.handle_change(self.info).await
    }
}
impl<'a, 'b, ENV: Environment> Iterator for Iter<'a, 'b, ENV> {
    type Item = RepoEntry<'a, 'b, ENV>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_, info)| RepoEntry {
            info,
            env: self.env,
        })
    }
}
impl<'a, 'b, ENV: Environment> FuzzKey for RepoEntry<'a, 'b, ENV> {
    fn fuzz_key<'c>(&'c self) -> std::borrow::Cow<'c, str> {
        self.info.fuzz_key()
    }
}