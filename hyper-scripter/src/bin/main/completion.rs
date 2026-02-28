use crate::def::{self, ID};
use clap::Parser;
use hyper_scripter::args::{AliasRoot, Root, RootArgs};
use hyper_scripter::config::Config;
use hyper_scripter::error::Error;
use hyper_scripter::error::Result;
use hyper_scripter::fuzzy::{fuzz_with_multifuzz_ratio, is_prefix, FuzzResult};
use hyper_scripter::path;
use hyper_scripter::script_repo::{RepoEntry, ScriptRepo, Visibility};
use hyper_scripter::util::{get_types, init_repo, main_util};
use hyper_scripter::{Either, SEP};
use std::cmp::Reverse;
use std::io::stdout;
use supplement::{Completion, CompletionGroup, History, Shell};

fn sort(v: &mut Vec<RepoEntry<'_>>) {
    v.sort_by_key(|s| Reverse(s.last_time()));
}
async fn fuzz_arr<'a>(
    name: &str,
    iter: impl Iterator<Item = RepoEntry<'a>>,
) -> Result<Vec<RepoEntry<'a>>> {
    // TODO: 測試這個複雜的函式，包括前綴和次級結果
    let res = fuzz_with_multifuzz_ratio(name, iter, SEP, Some(60)).await?;
    Ok(match res {
        None => vec![],
        Some(FuzzResult::Single(t)) => vec![t.obj],
        Some(FuzzResult::Multi {
            ans,
            others,
            still_others,
        }) => {
            let prefix = ans.obj.name.key();
            let mut first_others = vec![];
            let mut prefixed_others = vec![];
            for candidate in others.into_iter() {
                if is_prefix(&*prefix, &*candidate.obj.name.key(), SEP) {
                    prefixed_others.push(candidate.obj);
                } else {
                    first_others.push(candidate.obj);
                }
            }
            first_others.push(ans.obj);

            sort(&mut first_others);
            sort(&mut prefixed_others);
            let mut still_others = still_others.into_iter().map(|t| t.obj).collect();
            sort(&mut still_others);
            first_others.append(&mut prefixed_others);
            first_others.append(&mut still_others);
            first_others
        }
    })
}

pub async fn handle_completion(args: Vec<String>, repo: &mut Option<ScriptRepo>) -> Result {
    let shell: Shell = args
        .get(2)
        .map(String::as_str)
        .unwrap_or_default()
        .parse()
        .map_err(|e| Error::Others(vec![e], None))?;
    let args = &args[3..];

    match AliasRoot::try_parse_from(args) {
        Err(e) => {
            log::warn!("展開別名時出錯 {}", e); // NOTE: -V 或 --help 也會走到這裡
        }
        Ok(root) if root.root_args.no_alias => log::info!("無別名模式"),
        Ok(root) if root.subcmd.len() == 1 => log::info!("別名為最後一位，不展開"),
        Ok(root) => {
            let home = path::compute_home_path_optional(root.root_args.hs_home.as_ref(), false)?;
            // TODO: we can try to make this `load` reused further
            let conf = Config::load(&home)?;
            if let Some(Either::One(new_args)) = root.expand_alias(args, &conf) {
                let args_iter = new_args.map(String::from);
                return handle_completion_no_alias(shell, args_iter, repo).await;
            };
        }
    }

    handle_completion_no_alias(shell, args.iter().map(String::from), repo).await
}

pub async fn handle_completion_no_alias(
    shell: Shell,
    args: impl Iterator<Item = String>,
    repo: &mut Option<ScriptRepo>,
) -> Result {
    let (history, grp) = def::CMD.supplement(args)?;
    let ready = match grp {
        CompletionGroup::Ready(r) => r,
        CompletionGroup::Unready { unready, id, value } => {
            let comps = handle_comp(id, history, &value, repo).await?;
            unready.to_ready(comps)
        }
    };
    ready.print(shell, &mut stdout())?;
    Ok(())
}

fn get_root(history: &History<ID>) -> Result<Root> {
    let select = history.find(def::ID_VAL_SELECT).map(|h| &h.values);
    let select: Vec<_> = select
        .iter()
        .flat_map(|v| v.iter())
        .map(|s| s.parse().unwrap())
        .collect();

    let root_args = RootArgs {
        select,
        dump_args: false,
        no_trace: false,
        humble: false,
        prompt_level: None,
        no_alias: history.find(def::ID_VAL_NO_ALIAS).is_some(),
        hs_home: history.find(def::ID_VAL_HS_HOME).map(|h| h.value.clone()),
        archaeology: history.find(def::ID_VAL_ARCHAEOLOGY).is_some(),
        all: history.find(def::ID_VAL_ALL).is_some(),
        timeless: history.find(def::ID_VAL_TIMELESS).is_some(),
        toggle: history
            .find(def::ID_VAL_TOGGLE)
            .map(|h| h.values.clone())
            .unwrap_or_default(),
        recent: history
            .find(def::ID_VAL_RECENT)
            .map(|r| r.value.parse().unwrap()),
    };

    log::info!("parsed root args: {root_args:?}");

    let root = Root::from_args(root_args);
    root.set_home_unless_from_alias(false, true)?;
    Ok(root)
}

async fn complete_script_with_root(
    mut value: &str,
    mut root: Root,
    repo: &mut Option<ScriptRepo>,
) -> Result<Vec<Completion>> {
    let bang = value.ends_with("!");
    let bang_str = if bang { "!" } else { "" };
    if bang {
        value = &value[..value.len() - 1];
    }

    root.sanitize_flags(bang);
    *repo = Some(init_repo(root.root_args, false).await?);
    let iter = repo.as_mut().unwrap().iter_mut(Visibility::Normal);

    let v = if value.is_empty() || value.starts_with("^") {
        // NOTE: Special case. Latest script completion.
        // TODO: Make it support anonymous script
        let mut v: Vec<_> = iter.collect();
        sort(&mut v);
        v.into_iter()
            .take(10)
            .enumerate()
            .map(|(i, s)| {
                let val = format!("^{}{}", i + 1, bang_str);
                let desc = s.name.key();
                Completion::new(val, desc).group("scripts")
            })
            .collect()
    } else {
        let exact = value.starts_with('=');
        if exact {
            value = &value[1..];
        }
        let exact_str = if exact { "=" } else { "" };

        let v = fuzz_arr(value, iter).await?;
        v.into_iter()
            .map(|s| {
                let name = format!("{}{}{}", exact_str, s.name.key(), bang_str);
                Completion::new(name.clone(), name).group("scripts")
            })
            .collect()
    };
    Ok(v)
}

async fn complete_script(
    value: &str,
    history: &History<ID>,
    repo: &mut Option<ScriptRepo>,
) -> Result<Vec<Completion>> {
    let root = get_root(history)?;
    complete_script_with_root(value, root, repo).await
}

fn list_types_with_root(sub_types: bool) -> Result<impl Iterator<Item = Completion>> {
    let types = get_types(sub_types)?;
    Ok(types
        .into_iter()
        .map(|ty| Completion::new(ty, "").group("types")))
}
fn list_types(history: &History<ID>, sub_types: bool) -> Result<impl Iterator<Item = Completion>> {
    get_root(history)?;
    list_types_with_root(sub_types)
}

macro_rules! id {
    ($($id:tt)+) => {
        supplement::helper::id!(def $($id )+)
    };
}

async fn handle_comp(
    id: ID,
    history: History<ID>,
    value: &str,
    repo: &mut Option<ScriptRepo>,
) -> Result<Vec<Completion>> {
    log::info!("handle completion for {id:?}");
    let v = match id {
        id!(recent)
        | id!(history show offset)
        | id!(history show limit)
        | id!(history rm range) => vec![],

        id!(hs_home) | id!(run dir) | id!(history show dir) | id!(history rm dir) => {
            Completion::files(value).collect()
        }

        id!(recent recent_filter) => {
            vec![
                Completion::new("no-neglect", ""),
                Completion::new("timeless", ""),
            ]
        }
        id!(@ext) => {
            if history.find(def::ID_EXTERNAL).is_some() {
                // Not the first position
                Completion::files(value).collect()
            } else {
                let root = get_root(&history)?;
                let no_alias = root.root_args.no_alias;
                let mut comps = complete_script_with_root(value, root, repo).await?;
                if !no_alias {
                    let aliases = Config::get().alias.iter().map(|(key, val)| {
                        let after = val.after.join(" ");
                        Completion::new(key, after).group("alias")
                    });
                    comps.extend(aliases);
                }
                comps
            }
        }
        id!(toggle) | id!(tags toggle names) | id!(tags set name) | id!(tags unset name) => {
            get_root(&history)?;
            Config::get()
                .tag_selectors
                .iter()
                .map(|s| Completion::new(&s.name, ""))
                .collect()
        }
        id!(select) => {
            let root = get_root(&history)?;
            *repo = Some(init_repo(root.root_args, false).await?);

            let types = list_types_with_root(false)?.map(|c| c.value(|v| format!("@{v}!")));
            let tags = main_util::known_tags_iter(repo.as_mut().unwrap())
                .map(|ty| Completion::new(format!("{ty}!"), "").group("tags"));
            let mut comps: Vec<_> = types.chain(tags).collect();
            comps.extend(
                comps
                    .clone()
                    .into_iter()
                    .map(|c| c.value(|v| format!("+{v}"))),
            );
            comps
        }
        id!(cat queries)
        | id!(which queries)
        | id!(edit edit_query)
        | id!(help args)
        | id!(mv origin)
        | id!(cp origin)
        | id!(ls queries)
        | id!(rm queries)
        | id!(run script_query)
        | id!(history show queries)
        | id!(history rm queries) => complete_script(value, &history, repo).await?,

        id!(mv ty) => list_types(&history, false)?.collect(),
        id!(edit ty) | id!(types ty) => list_types(&history, true)?.collect(),

        _ => unimplemented!("id = {id:?}"),
    };

    Ok(v)
}
