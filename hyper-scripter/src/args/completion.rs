use super::NO_FLAG_SETTINGS;
use crate::error::{Error, Result};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Completion {
    #[structopt(settings = NO_FLAG_SETTINGS)]
    LS { args: Vec<String> },
    #[structopt(settings = NO_FLAG_SETTINGS)]
    Alias { args: Vec<String> },
}

impl Completion {
    pub fn from_args(args: &[String]) -> Result<Option<Completion>> {
        let args = &args[1..];
        if args.first().map(AsRef::as_ref) == Some("completion") {
            log::info!("補全模式 {:?}", args);
            match Completion::from_iter_safe(args) {
                Ok(c) => Ok(Some(c)),
                Err(e) => {
                    log::warn!("解析補全參數出錯：{}", e);
                    Err(Error::Completion)
                }
            }
        } else {
            Ok(None)
        }
    }
}
