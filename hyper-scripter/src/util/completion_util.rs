use super::{init_repo, print_iter};
use crate::args::{AliasRoot, Completion, Root, Subs};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::path;
use crate::query::do_list_query;
use std::cmp::Reverse;
use structopt::StructOpt;

pub async fn handle_completion(comp: Completion) -> Result {
    match comp {
        Completion::LS { args } => {
            let new_root = match Root::from_iter_safe(args) {
                Ok(Root {
                    subcmd: Some(Subs::Tags(_)),
                    ..
                }) => {
                    // TODO: 在補全腳本中處理，而不要在這邊
                    return Err(Error::Completion);
                }
                Ok(t) => t,
                Err(e) => {
                    log::warn!("補全時出錯 {}", e);
                    // NOTE: -V 或 --help 也會走到這裡
                    return Err(Error::Completion);
                }
            };
            log::info!("補完模式，參數為 {:?}", new_root);
            new_root.set_home_unless_set()?;
            let mut repo = init_repo(new_root.root_args, false).await?;

            let mut scripts = do_list_query(&mut repo, &[]).await?;
            scripts.sort_by_key(|s| Reverse(s.last_time()));
            print_iter(scripts.iter().map(|s| s.name.key()), " ");

            Ok(())
        }
        Completion::Alias { args } => {
            match AliasRoot::from_iter_safe(&args) {
                Ok(alias_root) => {
                    fn print_iter<T: std::fmt::Display>(iter: impl Iterator<Item = T>) {
                        for arg in iter {
                            print!("{} ", arg);
                        }
                    }

                    let p =
                        path::compute_home_path_optional(alias_root.root_args.hs_home.as_ref())?;
                    let conf = Config::load(&p)?;
                    if let Some(new_args) = alias_root.expand_alias(&args, &conf) {
                        print_iter(new_args);
                    } else {
                        print_iter(args.iter());
                    };
                    Ok(())
                }
                Err(e) => {
                    log::warn!("展開別名時出錯 {}", e);
                    // NOTE: -V 或 --help 也會走到這裡
                    Err(Error::Completion)
                }
            }
        }
    }
}
