use crate::error::Result;
use crate::fuzzy::FuzzKey;
use crate::script::ScriptInfo;

use super::DBEnv;

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

impl<'b> FuzzKey for RepoEntry<'b> {
    fn fuzz_key(&self) -> std::borrow::Cow<'_, str> {
        self.info.name.key()
    }
}
