use crate::args::RootArgs;
use crate::error::Result;
use crate::script_repo::{DBEnv, ScriptRepo};
use crate::{db, util};
use hyper_scripter_historian::Historian;

pub enum Resource {
    None,
    Repo(ScriptRepo),
    Historian(Historian),
    Env(DBEnv),
}
impl Resource {
    pub async fn close(self) {
        match self {
            Self::None => (),
            Self::Repo(repo) => repo.close().await,
            Self::Historian(historian) => historian.close().await,
            Self::Env(env) => env.close().await,
        }
    }
}

pub struct RepoHolder<'a> {
    pub root_args: RootArgs,
    pub need_journal: bool,
    pub resource: &'a mut Resource,
}

impl<'a> RepoHolder<'a> {
    pub async fn init(self) -> Result<&'a mut ScriptRepo> {
        let repo = util::init_repo(self.root_args, self.need_journal).await?;
        *self.resource = Resource::Repo(repo);
        match self.resource {
            Resource::Repo(repo) => Ok(repo),
            _ => unreachable!(),
        }
    }
    pub async fn historian(self) -> Result<&'a mut Historian> {
        let historian = Historian::new(db::get_history_file()).await?;
        *self.resource = Resource::Historian(historian);
        match self.resource {
            Resource::Historian(historian) => Ok(historian),
            _ => unreachable!(),
        }
    }
    pub async fn env(self) -> Result<&'a mut DBEnv> {
        let (env, init) = util::init_env(self.need_journal).await?;
        if init {
            log::error!("還沒初始化就想做進階操作 ==");
            std::process::exit(0);
        }
        *self.resource = Resource::Env(env);
        match self.resource {
            Resource::Env(env) => Ok(env),
            _ => unreachable!(),
        }
    }
}
