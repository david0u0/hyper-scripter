use clap::Parser;
use hyper_scripter::args::{AliasRoot, History, List, Root, RootArgs, Subs, Tags, Types};
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
use supplement::{Completion, CompletionGroup, Seen, Shell, Supplement};

// FIXME: Every `init_repo` here may create DB file if there isn't one. Shouldn't do that.

fn empty(value: impl ToString) -> Completion {
    Completion::new(value, "")
}

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
        Ok(root) if root.subcmd.len() == 0 => log::info!("沒有別名，不展開"),
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
    let (history, grp) = Root::supplement(args)?;
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

type ID = <Root as Supplement>::ID;
fn get_root(ctx: ID, seen: &Seen) -> Result<Root> {
    let ctx = ctx.root_args;
    let select = ctx.select(seen).map(|s| s.unwrap()).collect();

    let root_args = RootArgs {
        select,
        dump_args: false,
        no_trace: false,
        humble: false,
        prompt_level: None,
        no_alias: ctx.no_alias(seen) != 0,
        hs_home: ctx.hs_home(seen).map(str::to_string),
        archaeology: ctx.archaeology(seen) != 0,
        all: ctx.all(seen) != 0,
        timeless: ctx.timeless(seen) != 0,
        toggle: ctx.toggle(seen).map(str::to_string).collect(),
        recent: ctx.recent(seen).map(|x| x.unwrap()),
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
                let prev = format!("^{}{}", i + 1, bang_str);
                let name = format!("{}{}", s.name.key(), bang_str);
                if value.is_empty() {
                    Completion::new(name, prev).group("scripts")
                } else {
                    Completion::new(prev, name).group("scripts")
                }
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
                // `always_match` to mimic the "reorder fuzzy" behavior
                Completion::new(name, value).group("scripts").always_match()
            })
            .collect()
    };
    Ok(v)
}

async fn complete_script(
    value: &str,
    ctx: ID,
    seen: &Seen,
    repo: &mut Option<ScriptRepo>,
) -> Result<Vec<Completion>> {
    let root = get_root(ctx, seen)?;
    complete_script_with_root(value, root, repo).await
}

fn list_types_with_root(sub_types: bool) -> Result<impl Iterator<Item = Completion>> {
    let types = get_types(sub_types)?;
    Ok(types.into_iter().map(|ty| empty(ty).group("types")))
}
fn list_types(ctx: ID, seen: &Seen, sub_types: bool) -> Result<impl Iterator<Item = Completion>> {
    get_root(ctx, seen)?;
    list_types_with_root(sub_types)
}

fn complete_alias() -> impl Iterator<Item = Completion> {
    Config::get().alias.iter().map(|(key, val)| {
        let after = val.after.join(" ");
        Completion::new(key, after).group("alias")
    })
}

fn prefix_plus(value: &str, mut comps: Vec<Completion>) -> Vec<Completion> {
    if !value.starts_with('+') && !value.is_empty() {
        return comps;
    }
    comps.extend(
        comps
            .clone()
            .into_iter()
            .map(|c| c.value(|v| format!("+{v}"))),
    );
    comps
}

macro_rules! id {
    ($($id:tt)+) => {
        supplement::helper::id!(Root . $($id )+)
    };
}

async fn handle_comp(
    id: ID,
    history: Seen,
    value: &str,
    repo: &mut Option<ScriptRepo>,
) -> Result<Vec<Completion>> {
    log::info!("handle completion for {id:?}");
    let v = match id {
        id!(root_args RootArgs.recent)
        | id!(subcmd Subs.Top.id)
        | id!(subcmd Subs.LS List.limit)
        | id!(subcmd Subs.LS List.format)
        | id!(subcmd Subs.Run.repeat)
        | id!(subcmd Subs.Cat.with)
        | id!(subcmd Subs.Edit.content)
        | id!(subcmd Subs.Alias.after)
        | id!(subcmd Subs.History.subcmd History.Show.offset)
        | id!(subcmd Subs.History.subcmd History.Show.limit)
        | id!(subcmd Subs.History.subcmd History.RM.range)
        | id!(subcmd Subs.History.subcmd History.Amend.event_id)
        | id!(subcmd Subs.History.subcmd History.Amend.env)
        | id!(subcmd Subs.History.subcmd History.RMID.event_id)
        | id!(subcmd Subs.History.subcmd History.Humble.event_id) => vec![],

        id!(root_args RootArgs.hs_home)
        | id!(subcmd Subs.Run.dir)
        | id!(subcmd Subs.Run.args)
        | id!(subcmd Subs.History.subcmd History.Show.dir)
        | id!(subcmd Subs.History.subcmd History.Amend.args) => std::process::exit(1),

        id!(subcmd Subs.Recent.recent_filter) => {
            vec![empty("no-neglect"), empty("timeless")]
        }

        id!(subcmd Subs.Other(ctx)) if ctx.values(&history).len() > 0 => {
            // Not the first position
            std::process::exit(1)
        }
        id!(subcmd Subs.Other(ctx)) => {
            let root = get_root(id, &history)?;
            let no_alias = root.root_args.no_alias;
            let mut comps = complete_script_with_root(value, root, repo).await?;
            if !no_alias {
                let aliases = complete_alias();
                comps.extend(aliases);
            }
            comps
        }
        id!(subcmd Subs.Alias.before) => {
            get_root(id, &history)?;
            complete_alias().collect()
        }
        id!(root_args RootArgs.toggle)
        | id!(subcmd Subs.Tags.subcmd Tags.Toggle.names)
        | id!(subcmd Subs.Tags.subcmd Tags.Set.name)
        | id!(subcmd Subs.Tags.subcmd Tags.Unset.name) => {
            get_root(id, &history)?;
            Config::get()
                .tag_selectors
                .iter()
                .map(|s| empty(&s.name))
                .collect()
        }
        id!(root_args RootArgs.select) => {
            let root = get_root(id, &history)?;
            *repo = Some(init_repo(root.root_args, false).await?);

            let types = list_types_with_root(false)?.map(|c| c.value(|v| format!("@{v}!")));
            let tags = main_util::known_tags_iter(repo.as_mut().unwrap())
                .map(|ty| empty(format!("{ty}!")).group("tags"));
            let comps: Vec<_> = types.chain(tags).collect();
            prefix_plus(value, comps)
        }
        id!(subcmd Subs.Other(ctx)) if ctx.values(&history).len() > 0 => {
            // Not the first position
            vec![]
        }
        id!(subcmd Subs.Other) => {
            let root = get_root(id, &history)?;
            *repo = Some(init_repo(root.root_args, false).await?);
            let tags = main_util::known_tags_iter(repo.as_mut().unwrap())
                .map(|ty| empty(format!("+{ty}")).group("tags"));
            tags.collect()
        }
        id!(subcmd Subs.Edit.tags) | id!(subcmd Subs.MV.tags) | id!(subcmd Subs.CP.tags) => {
            let root = get_root(id, &history)?;
            *repo = Some(init_repo(root.root_args, false).await?);
            let tags = main_util::known_tags_iter(repo.as_mut().unwrap())
                .map(|ty| empty(ty).group("tags"));
            let tags: Vec<_> = tags.collect();
            prefix_plus(value, tags)
        }
        id!(subcmd Subs.Cat.queries)
        | id!(subcmd Subs.Which.queries)
        | id!(subcmd Subs.Edit.edit_query)
        | id!(subcmd Subs.Help.args)
        | id!(subcmd Subs.MV.origin)
        | id!(subcmd Subs.CP.origin)
        | id!(subcmd Subs.MV.new)
        | id!(subcmd Subs.CP.new)
        | id!(subcmd Subs.LS List.queries)
        | id!(subcmd Subs.RM.queries)
        | id!(subcmd Subs.Run.script_query)
        | id!(subcmd Subs.History.subcmd History.Neglect.queries)
        | id!(subcmd Subs.History.subcmd History.Show.queries)
        | id!(subcmd Subs.History.subcmd History.RM.queries)
        | id!(subcmd Subs.Top.queries) => complete_script(value, id, &history, repo).await?,

        id!(subcmd Subs.MV.ty) => list_types(id, &history, false)?.collect(),
        id!(subcmd Subs.Edit.ty) | id!(subcmd Subs.Types Types.ty) => {
            list_types(id, &history, true)?.collect()
        }

        _ => todo!(),
    };

    Ok(v)
}
