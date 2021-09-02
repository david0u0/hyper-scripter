use crate::error::Result;
use crate::fuzzy::FuzzKey;
use crate::script::ScriptInfo;
use std::collections::hash_map::IterMut as HashMapIter;

use super::DBEnv;

pub struct Iter<'b> {
    pub(super) iter: HashMapIter<'b, String, ScriptInfo>,
    pub(super) iter2: Option<HashMapIter<'b, String, ScriptInfo>>,
    pub(super) env: &'b DBEnv,
}
#[derive(Deref, Debug)]
pub struct RepoEntry<'b> {
    #[deref]
    pub(super) info: &'b mut ScriptInfo,
    pub(super) env: &'b DBEnv, // XXX: 一旦 async trait 可用了就把這裡變成 trait，不要對實作編程
}

impl<'b> RepoEntry<'b> {
    pub(super) fn new(info: &'b mut ScriptInfo, env: &'b DBEnv) -> Self {
        RepoEntry { info, env }
    }
    /// 回傳值為「上一筆記錄到的事件的 id」
    pub async fn update<F: FnOnce(&mut ScriptInfo)>(&mut self, handler: F) -> Result<i64> {
        handler(self.info);
        let last_event_id = self.env.handle_change(self.info).await?;
        Ok(last_event_id)
    }
    pub fn into_inner(self) -> &'b ScriptInfo {
        self.info
    }
    pub fn get_env(&self) -> &DBEnv {
        self.env
    }
}
impl<'b> Iterator for Iter<'b> {
    type Item = RepoEntry<'b>;
    fn next(&mut self) -> Option<Self::Item> {
        // TODO: 似乎有優化空間？參考標準庫 Chain
        if let Some((_, info)) = self.iter.next() {
            Some(RepoEntry::new(info, self.env))
        } else if let Some(iter) = self.iter2.as_mut() {
            iter.next().map(|(_, info)| RepoEntry::new(info, self.env))
        } else {
            None
        }
    }
}

impl<'b> FuzzKey for RepoEntry<'b> {
    fn fuzz_key(&self) -> std::borrow::Cow<'_, str> {
        self.info.name.key()
    }
}
