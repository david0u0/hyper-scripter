use super::{init_repo, print_iter, to_display_args};
use crate::args::{AliasRoot, Completion, Root, Subs};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::fuzzy::{fuzz_with_multifuzz_ratio, is_prefix, FuzzResult};
use crate::path;
use crate::script_repo::{RepoEntry, Visibility};
use crate::SEP;
use std::cmp::Reverse;
use structopt::StructOpt;

fn sort(v: &mut Vec<RepoEntry<'_>>) {
    v.sort_by_key(|s| Reverse(s.last_time()));
}

fn parse_alias_root(args: &[String]) -> Result<AliasRoot> {
    match AliasRoot::from_iter_safe(args) {
        Ok(root) => Ok(root),
        Err(e) => {
            log::warn!("展開別名時出錯 {}", e);
            // NOTE: -V 或 --help 也會走到這裡
            Err(Error::Completion)
        }
    }
}

async fn fuzz_arr<'a>(
    name: &str,
    iter: impl Iterator<Item = RepoEntry<'a>>,
) -> Result<Vec<RepoEntry<'a>>> {
    // TODO: 測試這個複雜的函式，包括前綴和次級結果
    let res = fuzz_with_multifuzz_ratio(name, iter, SEP, Some(60)).await?;
    Ok(match res {
        None => vec![],
        Some(FuzzResult::High(t) | FuzzResult::Low(t)) => vec![t],
        Some(FuzzResult::Multi {
            ans,
            others,
            mut still_others,
        }) => {
            let prefix = ans.name.key();
            let mut first_others = vec![];
            let mut prefixed_others = vec![];
            for candidate in others.into_iter() {
                if is_prefix(&*prefix, &*candidate.name.key(), SEP) {
                    prefixed_others.push(candidate);
                } else {
                    first_others.push(candidate);
                }
            }
            first_others.push(ans);

            sort(&mut first_others);
            sort(&mut prefixed_others);
            sort(&mut still_others);
            first_others.append(&mut prefixed_others);
            first_others.append(&mut still_others);
            first_others
        }
    })
}

pub async fn handle_completion(comp: Completion) -> Result {
    match comp {
        Completion::LS { name, args } => {
            let mut new_root = match Root::from_iter_safe(args) {
                Ok(Root {
                    subcmd: Some(Subs::Tags(_) | Subs::Types(_)),
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
            new_root.set_home_unless_from_alias(false)?;
            new_root.sanitize_flags();
            let mut repo = init_repo(new_root.root_args, false).await?;

            let iter = repo.iter_mut(Visibility::Normal);
            let scripts = if let Some(name) = name {
                fuzz_arr(&name, iter).await?
            } else {
                let mut t: Vec<_> = iter.collect();
                sort(&mut t);
                t
            };

            print_iter(scripts.iter().map(|s| s.name.key()), " ");
        }
        Completion::NoSubcommand { args } => {
            if let Ok(root) = parse_alias_root(&args) {
                if root.subcmd.is_some() {
                    return Err(Error::Completion);
                }
            } // else: 解析錯誤當然不可能有子命令啦
        }
        Completion::Alias { args } => {
            let root = parse_alias_root(&args)?;
            let home = path::compute_home_path_optional(root.root_args.hs_home.as_ref(), false)?;
            let conf = Config::load(&home)?;
            if let Some(new_args) = root.expand_alias(&args, &conf) {
                print_iter(new_args, " ");
            } else {
                log::info!("並非別名");
                return Err(Error::Completion);
            };
        }
        Completion::Home { args } => {
            let root = parse_alias_root(&args)?;
            let home = root.root_args.hs_home.ok_or_else(|| Error::Completion)?;
            print!("{}", home);
        }
        Completion::ParseRun { args } => {
            let mut root = Root::from_iter_safe(args).map_err(|e| {
                log::warn!("補全時出錯 {}", e);
                Error::Completion
            })?;
            root.sanitize()?;
            match root.subcmd {
                Some(Subs::Run {
                    script_query, args, ..
                }) => {
                    let iter = std::iter::once(script_query.to_string())
                        .chain(args.into_iter().map(|s| to_display_args(s)));
                    print_iter(iter, " ");
                }
                res @ _ => {
                    log::warn!("非執行指令 {:?}", res);
                    return Err(Error::Completion);
                }
            }
        }
    }
    Ok(())
}
