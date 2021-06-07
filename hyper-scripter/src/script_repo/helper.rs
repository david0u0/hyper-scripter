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
    /// 記錄「上一筆創造出來的事件」的 id
    pub(super) last_event_id: i64,
    #[deref]
    pub(super) info: &'b mut ScriptInfo,
    pub(super) env: &'b DBEnv, // XXX: 一旦 async trait 可用了就把這裡變成 trait，不要對實作編程
}

impl<'b> RepoEntry<'b> {
    pub fn last_event_id(&self) -> i64 {
        self.last_event_id
    }
    pub(super) fn new(info: &'b mut ScriptInfo, env: &'b DBEnv) -> Self {
        RepoEntry {
            info,
            env,
            last_event_id: 0,
        }
    }
    pub async fn update<F: FnOnce(&mut ScriptInfo)>(&mut self, handler: F) -> Result {
        handler(self.info);
        let last_event_id = self.env.handle_change(self.info).await?;
        self.last_event_id = last_event_id;
        Ok(())
    }
    pub fn into_inner(self) -> &'b ScriptInfo {
        self.info
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
