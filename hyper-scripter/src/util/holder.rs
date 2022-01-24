use crate::args::RootArgs;
use crate::error::Result;
use crate::script_repo::{DBEnv, ScriptRepo};
use crate::{path, util};
use hyper_scripter_historian::Historian;

pub struct RepoHolder {
    pub root_args: RootArgs,
    pub need_journal: bool,
}

pub struct RepoCloser;
pub struct HistorianCloser;
pub struct EnvCloser;

impl RepoHolder {
    pub async fn init(self) -> Result<(ScriptRepo, RepoCloser)> {
        Ok((
            util::init_repo(self.root_args, self.need_journal).await?,
            RepoCloser,
        ))
    }
    pub async fn historian(self) -> Result<(Historian, HistorianCloser)> {
        let historian = Historian::new(path::get_home().to_owned()).await?;
        Ok((historian, HistorianCloser))
    }
    pub async fn env(self) -> Result<(DBEnv, EnvCloser)> {
        let (env, init) = util::init_env(self.need_journal).await?;
        if init {
            log::error!("還沒初始化就想做進階操作 ==");
            std::process::exit(0);
        }
        Ok((env, EnvCloser))
    }
}

impl RepoCloser {
    pub async fn close(self, repo: ScriptRepo) {
        repo.close().await
    }
}
impl HistorianCloser {
    pub async fn close(self, historian: Historian) {
        historian.close().await
    }
}
impl EnvCloser {
    pub async fn close(self, env: DBEnv) {
        env.close().await
    }
}
