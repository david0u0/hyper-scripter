use fxhash::FxHashMap as HashMap;
use hyper_scripter::args::{self, History, List, Root, Subs, Tags, TagsSubs, Types, TypesSubs};
use hyper_scripter::config::{Config, NamedTagFilter};
use hyper_scripter::error::{Error, RedundantOpt, Result};
use hyper_scripter::extract_msg::{extract_env_from_content, extract_help_from_content};
use hyper_scripter::list::{fmt_list, DisplayIdentStyle, DisplayStyle, ListOptions};
use hyper_scripter::path;
use hyper_scripter::query::{self, ScriptQuery};
use hyper_scripter::script_repo::{RepoEntry, ScriptRepo};
use hyper_scripter::script_time::ScriptTime;
use hyper_scripter::tag::{Tag, TagFilter};
use hyper_scripter::util::{
    self, completion_util,
    holder::RepoHolder,
    main_util::{self, EditTagArgs},
    print_iter,
};
use hyper_scripter::Either;
use hyper_scripter::{db, migration};
use hyper_scripter_historian::{Historian, LastTimeRecord};

#[tokio::main]
async fn main() {
    env_logger::init();
    let errs = match main_err_handle().await {
        Err(e) => vec![e],
        Ok(v) => v,
    };
    let mut exit_code = 0;
    for err in errs.iter() {
        use Error::*;
        match err {
            ScriptError(c) | PreRunError(c) | EditorError(c, _) => exit_code = *c,
            _ => {
                if exit_code == 0 {
                    exit_code = 1;
                }
            }
        }
        eprint!("{}", err);
    }
    std::process::exit(exit_code);
}
async fn main_err_handle() -> Result<Vec<Error>> {
    let args: Vec<_> = std::env::args().collect();
    let root = args::handle_args(args)?;
    let root = match root {
        Either::One(root) => root,
        Either::Two(comp) => {
            completion_util::handle_completion(comp).await?;
            std::process::exit(0);
        }
    };
    if root.root_args.dump_args {
        let dumped = serde_json::to_string(&root)?;
        print!("{}", dumped);
        return Ok(vec![]);
    }

    root.set_home_unless_from_alias(true)?;

    if matches!(root.subcmd, Some(Subs::Migrate)) {
        migration::do_migrate(db::get_file()).await?;
        Historian::do_migrate(path::get_home()).await?;
        return Ok(vec![]);
    }

    let res = main_inner(root).await?;
    if let Some(conf) = res.conf {
        log::info!("存入改變後的設定檔");
        conf.store()?;
    } else if Config::get().is_from_dafault() {
        log::info!("存入憑空產生的設定檔");
        Config::get().store()?;
    }
    Ok(res.errs)
}

struct MainReturn {
    conf: Option<Config>,
    /// 用來裝那種不會馬上造成中止的錯誤，例如 ScriptError
    errs: Vec<Error>,
}

async fn main_inner(root: Root) -> Result<MainReturn> {
    Config::set_prompt_level(root.root_args.prompt_level);
    let explicit_filter = !root.root_args.filter.is_empty();

    let conf = Config::get();
    let need_journal = main_util::need_write(root.subcmd.as_ref().unwrap());
    let repo = RepoHolder {
        root_args: root.root_args,
        need_journal,
    };

    let mut ret = MainReturn {
        conf: None,
        errs: vec![],
    };

    macro_rules! conf_mut {
        () => {{
            ret.conf = Some(conf.clone());
            ret.conf.as_mut().unwrap()
        }};
    }

    match root.subcmd.unwrap() {
        Subs::LoadUtils => {
            let (mut repo, closer) = repo.init().await?;
            main_util::load_utils(&mut repo).await?;
            closer.close(repo).await;
        }
        Subs::Alias {
            unset: false,
            before: Some(before),
            after,
            ..
        } => {
            if !after.is_empty() {
                log::info!("設定別名 {} {:?}", before, after);
                let conf = conf_mut!();
                conf.alias.insert(before, after.into());
            } else {
                log::info!("印出別名 {}", before);
                let after = conf
                    .alias
                    .get(&before)
                    .ok_or_else(|| Error::NoAlias(before.clone()))?
                    .after
                    .join(" ");
                println!("{}=\"{}\"", before, after);
            }
        }
        Subs::Alias {
            unset: true,
            before: Some(before),
            ..
        } => {
            log::info!("取消別名 {}", before);
            let conf = conf_mut!();
            let ok = conf.alias.remove(&before).is_some();
            if !ok {
                return Err(Error::NoAlias(before));
            }
        }
        Subs::Alias {
            unset: false,
            before: None,
            short,
            ..
        } => {
            log::info!("印出所有別名");
            for (before, alias) in conf.alias.iter() {
                print!("{}", before);
                if !short {
                    let after = alias.after.join(" ");
                    print!("=\"{}\"", after);
                }
                println!("");
            }
        }
        Subs::Edit {
            edit_query,
            ty,
            fast,
            tags,
            content,
            no_template,
        } => {
            // TODO: 這裡邏輯太複雜了，抽出來測試吧
            let (mut repo, closer) = repo.init().await?;
            let edit_tags = {
                // TODO: 不要這麼愛 clone
                let mut innate_tags = conf.main_tag_filter.clone();
                if let Some(tags) = tags {
                    let append_namespace = tags.append;
                    innate_tags.push(tags);
                    EditTagArgs {
                        explicit_filter,
                        explicit_tag: true,
                        content: innate_tags,
                        append_namespace,
                    }
                } else {
                    EditTagArgs {
                        explicit_filter,
                        explicit_tag: false,
                        append_namespace: true,
                        content: innate_tags,
                    }
                }
            };
            let (path, mut entry) =
                main_util::edit_or_create(edit_query, &mut repo, ty, edit_tags).await?;
            let prepare_resp = util::prepare_script(&path, &*entry, no_template, &content)?;
            entry.update(|info| info.read()).await?;
            if !fast {
                let res = util::open_editor(&path);
                if let Err(err) = res {
                    ret.errs.push(err);
                }
            }
            let should_keep = main_util::after_script(&mut entry, &path, &prepare_resp).await?;
            if !should_keep {
                util::remove(&path)?;
                let id = entry.id;
                repo.remove(id).await?
            }
            closer.close(repo).await;
        }
        Subs::Help { args } => {
            let (mut repo, closer) = repo.init().await?;
            let script_query: ScriptQuery = args[0].parse()?;
            let mut entry = query::do_script_query_strict(&script_query, &mut repo).await?;
            log::info!("檢視用法： {:?}", entry.name);
            create_read_event(&mut entry).await?;
            let script_path = path::open_script(&entry.name, &entry.ty, Some(true))?;
            let content = util::read_file(&script_path)?;

            let helps = extract_help_from_content(&content);
            let has_help = print_iter(helps, "\n");

            let mut envs = extract_env_from_content(&content).peekable();
            if envs.peek().is_some() {
                if has_help {
                    println!("\n");
                }
                print_iter(envs, "\n");
            }
            closer.close(repo).await;
        }
        Subs::Run {
            script_query,
            dummy,
            args,
            previous_args,
            error_no_previous,
            repeat,
            dir,
        } => {
            let (mut repo, closer) = repo.init().await?;
            let mut entry = query::do_script_query_strict(&script_query, &mut repo).await?;
            main_util::run_n_times(
                repeat.unwrap_or(1),
                dummy,
                &mut entry,
                args,
                &mut ret.errs,
                previous_args,
                error_no_previous,
                dir,
            )
            .await?;
            closer.close(repo).await;
        }
        Subs::Which { script_query } => {
            let (mut repo, closer) = repo.init().await?;
            let entry = query::do_script_query_strict(&script_query, &mut repo).await?;
            log::info!("定位 {:?}", entry.name);
            // NOTE: 不檢查存在與否
            let p = path::get_home().join(entry.file_path_fallback());
            println!("{}", p.to_string_lossy());
            closer.close(repo).await;
        }
        Subs::Cat { script_query } => {
            let (mut repo, closer) = repo.init().await?;
            let mut entry = query::do_script_query_strict(&script_query, &mut repo).await?;
            log::info!("打印 {:?}", entry.name);
            let script_path = path::open_script(&entry.name, &entry.ty, Some(true))?;
            let content = util::read_file(&script_path)?;
            print!("{}", content);
            create_read_event(&mut entry).await?;
            closer.close(repo).await;
        }
        Subs::EnvHelp { script_query } => {
            let (mut repo, closer) = repo.init().await?;
            let mut entry = query::do_script_query_strict(&script_query, &mut repo).await?;
            log::info!("打印 {:?} 的環境變數", entry.name);
            create_read_event(&mut entry).await?;
            let script_path = path::open_script(&entry.name, &entry.ty, Some(true))?;
            let content = util::read_file(&script_path)?;
            let envs = extract_env_from_content(&content);
            print_iter(envs, "\n");
            closer.close(repo).await;
        }
        Subs::Types(Types {
            subcmd: Some(TypesSubs::LS),
        }) => {
            print_iter(conf.types.keys(), " ");
        }
        Subs::Types(Types {
            subcmd: Some(TypesSubs::Template { ty, edit }),
        }) => {
            if edit {
                let tmpl_path = util::get_template_path(&ty, false)?;
                util::open_editor(&tmpl_path)?;
            } else {
                let template = util::get_or_create_tamplate(&ty, false)?;
                println!("{}", template);
            }
        }
        Subs::LS(List {
            long,
            grouping,
            queries,
            plain,
            name,
            file,
        }) => {
            let display_style = match (long, file, name) {
                (false, true, false) => DisplayStyle::Short(DisplayIdentStyle::File, ()),
                (false, false, true) => DisplayStyle::Short(DisplayIdentStyle::Name, ()),
                (false, false, false) => DisplayStyle::Short(DisplayIdentStyle::Normal, ()),
                (false, true, true) => DisplayStyle::Short(DisplayIdentStyle::NameAndFile, ()),
                (true, false, false) => DisplayStyle::Long(()),
                _ => unreachable!(),
            };
            let opt = ListOptions {
                grouping: grouping.into(),
                plain,
                queries,
                display_style,
            };
            let stdout = std::io::stdout();
            let (mut repo, closer) = repo.init().await?;
            fmt_list(&mut stdout.lock(), &mut repo, opt).await?;
            closer.close(repo).await;
        }
        Subs::RM { queries, purge } => {
            let (mut repo, closer) = repo.init().await?;
            let delete_tag: Option<TagFilter> = Some("+removed".parse().unwrap());
            let mut to_purge = vec![]; // (name, ty, id)
            for mut entry in query::do_list_query(&mut repo, &queries).await?.into_iter() {
                log::info!("刪除 {:?}", *entry);
                if purge {
                    log::debug!("真的刪除腳本！");
                    to_purge.push((entry.name.clone(), entry.ty.clone(), entry.id));
                } else {
                    log::debug!("不要真的刪除腳本，改用標籤隱藏之：{:?}", entry.name);
                    let res = main_util::mv(&mut entry, None, None, delete_tag.clone()).await;
                    match res {
                        Err(Error::PathNotFound(_)) => {
                            log::warn!("{:?} 實體不存在，消滅之", entry.name);
                            to_purge.push((entry.name.clone(), entry.ty.clone(), entry.id));
                        }
                        Err(e) => return Err(e),
                        _ => (),
                    }
                }
            }
            for (name, ty, id) in to_purge.into_iter() {
                let p = path::open_script(&name, &ty, None)?;
                repo.remove(id).await?;
                if let Err(e) = util::remove(&p) {
                    log::warn!("刪除腳本實體遭遇錯誤：{}", e);
                }
            }
            closer.close(repo).await;
        }
        Subs::CP { origin, new, tags } => {
            let (mut repo, closer) = repo.init().await?;
            if repo.get_mut(&new, true).is_some() {
                return Err(Error::ScriptExist(new.to_string()));
            }
            let entry = query::do_script_query_strict(&origin, &mut repo).await?;
            let og_script = path::open_script(&entry.name, &entry.ty, Some(true))?;
            let new_script = path::open_script(&new, &entry.ty, Some(false))?;
            util::cp(&og_script, &new_script)?;
            let mut new_info = entry.cp(new.clone());

            if let Some(tags) = tags {
                // TODO: delete tag
                if tags.append {
                    log::debug!("附加上標籤：{:?}", tags);
                    new_info.tags.extend(tags.into_allowed_iter());
                } else {
                    log::debug!("設定標籤：{:?}", tags);
                    new_info.tags = tags.into_allowed_iter().collect();
                }
            }

            repo.entry(&new).or_insert(new_info).await?;
            closer.close(repo).await;
        }
        Subs::MV {
            origin,
            new,
            tags,
            ty,
        } => {
            let (mut repo, closer) = repo.init().await?;
            let new_name = match new {
                Some(name) => {
                    if repo.get_mut(&name, true).is_some() {
                        return Err(Error::ScriptExist(name.to_string()));
                    }
                    Some(name)
                }
                None => None,
            };
            let mut scripts = query::do_list_query(&mut repo, &[origin]).await?;
            if new_name.is_some() {
                if scripts.len() > 1 {
                    log::warn!("試圖把多個腳本移動成同一個");
                    return Err(RedundantOpt::Scripts(
                        scripts.iter().map(|s| s.name.key().to_string()).collect(),
                    )
                    .into());
                }
                //  只有一個，放心移動
                main_util::mv(&mut scripts[0], new_name, ty.clone(), tags.clone()).await?;
            } else {
                for entry in scripts.iter_mut() {
                    main_util::mv(entry, None, ty.clone(), tags.clone()).await?;
                }
            }
            closer.close(repo).await;
        }
        Subs::Tags(Tags {
            subcmd: Some(TagsSubs::Unset { name }),
        }) => {
            let conf = conf_mut!();
            let pos = conf
                .tag_filters
                .iter()
                .position(|f| f.name == name)
                .ok_or_else(|| {
                    log::error!("試著刪除不存在的篩選器 {:?}", name);
                    Error::TagFilterNotFound(name)
                })?;
            conf.tag_filters.remove(pos);
        }
        Subs::Tags(Tags {
            subcmd: Some(TagsSubs::Toggle { name }),
        }) => {
            let conf = conf_mut!();
            let filter = conf
                .tag_filters
                .iter_mut()
                .find(|f| f.name == name)
                .ok_or_else(|| {
                    log::error!("試著切換不存在的篩選器 {:?}", name);
                    Error::TagFilterNotFound(name)
                })?;
            filter.inactivated = !filter.inactivated;
        }
        Subs::Tags(Tags {
            subcmd:
                Some(TagsSubs::LS {
                    named: true,
                    known: false,
                }),
        }) => {
            print_iter(conf.tag_filters.iter().map(|f| &f.name), " ");
        }
        Subs::Tags(Tags {
            subcmd:
                Some(TagsSubs::LS {
                    named: false,
                    known,
                }),
        }) => {
            let (mut repo, closer) = repo.init().await?;
            if known {
                print_iter(known_tags_iter(&mut repo), " ");
            } else {
                print!("known tags:\n  ");
                print_iter(known_tags_iter(&mut repo), " ");
                println!("");
                println!("tag filters:");
                for filter in conf.tag_filters.iter() {
                    let content = &filter.content;
                    print!("  {} = [{}]", filter.name, content);
                    if content.mandatory {
                        print!(" (mandatory)")
                    }
                    if filter.inactivated {
                        print!(" (inactivated)")
                    }
                    println!()
                }
                println!("main tag filter:");
                print!("  [{}]", conf.main_tag_filter);
                if conf.main_tag_filter.mandatory {
                    print!(" (mandatory)")
                }
                println!();
            }
            closer.close(repo).await;
        }
        Subs::Tags(Tags {
            subcmd: Some(TagsSubs::Set { content, name }),
        }) => {
            let conf = conf_mut!();
            if let Some(name) = name {
                log::debug!("處理篩選器 {:?}", name);
                let tag_filters: &mut Vec<NamedTagFilter> = &mut conf.tag_filters;
                if let Some(existing_filter) = tag_filters.iter_mut().find(|f| f.name == name) {
                    log::info!("修改篩選器 {} {}", name, content);
                    existing_filter.content = content;
                } else {
                    log::info!("新增篩選器 {} {}", name, content);
                    tag_filters.push(NamedTagFilter {
                        content: content,
                        name,
                        inactivated: false,
                    });
                }
            } else {
                log::info!("加入主篩選器 {:?}", content);
                conf.main_tag_filter = content;
            }
        }
        Subs::History {
            subcmd: History::RM { queries, range },
        } => {
            let (mut repo, closer) = repo.init().await?;
            let historian = repo.historian().clone();
            let mut scripts = query::do_list_query(&mut repo, &queries).await?;
            let ids: Vec<_> = scripts.iter().map(|s| s.id).collect();
            let res_vec = historian
                .ignore_args_range(&ids, range.get_min(), range.get_max())
                .await?;
            // TODO: 測試多個腳本的狀況
            for (entry, res) in scripts.iter_mut().zip(res_vec) {
                // TODO: 平行？
                if check_time_changed(&entry, &res) {
                    log::debug!(
                        "刪除後時間不同 {:?} {:?} {:?} v.s. {:?}",
                        entry.exec_time,
                        entry.exec_done_time,
                        entry.humble_time,
                        res
                    );
                    entry
                        .update(|info| {
                            info.exec_time = res.exec_time.map(|t| ScriptTime::new(t));
                            info.exec_done_time = res.exec_done_time.map(|t| ScriptTime::new(t));
                            info.humble_time = res.humble_time;
                        })
                        .await?;
                }
            }
            closer.close(repo).await;
        }
        Subs::History {
            subcmd: History::RMID { event_id },
        } => {
            process_event_by_id(false, repo, event_id).await?;
        }
        Subs::History {
            subcmd: History::Humble { event_id },
        } => {
            process_event_by_id(true, repo, event_id).await?;
        }
        Subs::History {
            subcmd: History::Amend { event_id, args },
        } => {
            let (historian, closer) = repo.historian().await?;
            let args = serde_json::to_string(&args)?;
            historian.amend_args_by_id(event_id as i64, &args).await?;
            closer.close(historian).await;
        }
        Subs::History {
            subcmd: History::Tidy { queries },
        } => {
            let (mut repo, closer) = repo.init().await?;
            let historian = repo.historian().clone();
            for entry in query::do_list_query(&mut repo, &queries).await?.into_iter() {
                historian.tidy(entry.id).await?;
            }
            closer.close(repo).await;
        }
        Subs::History {
            subcmd: History::Neglect { queries },
        } => {
            let (mut repo, closer) = repo.init().await?;
            for entry in query::do_list_query(&mut repo, &queries).await?.into_iter() {
                let id = entry.id;
                entry.get_env().handle_neglect(id).await?;
            }
            closer.close(repo).await;
        }
        Subs::History {
            subcmd:
                History::Show {
                    queries,
                    limit,
                    with_name,
                    offset,
                    dir,
                },
        } => {
            let (mut repo, closer) = repo.init().await?;
            let historian = repo.historian().clone();
            let dir = util::option_map_res(dir, |d| path::normalize_path(d))?;
            let scripts = query::do_list_query(&mut repo, &queries).await?;
            let ids: Vec<_> = scripts.iter().map(|s| s.id).collect();
            let args_list = historian
                .previous_args_list(&ids, limit, offset, dir.as_deref())
                .await?;
            for (script_id, args) in args_list {
                log::debug!("嘗試打印參數 {} {}", script_id, args);
                let args: Vec<String> = serde_json::from_str(&args)?;
                if with_name {
                    let entry = repo.get_mut_by_id(script_id).ok_or_else(|| {
                        log::error!("史學家給的腳本 id 竟然在倉庫中找不到……");
                        Error::ScriptNotFound(script_id.to_string())
                    })?;
                    print!("{}", entry.name.key());
                    if !args.is_empty() {
                        print!(" ");
                    }
                }
                print_iter(args.into_iter().map(|s| util::to_display_args(s)), " ");
                println!("");
            }
            closer.close(repo).await;
        }
        sub => unimplemented!("{:?}", sub),
    }
    Ok(ret)
}

async fn create_read_event(entry: &mut RepoEntry<'_>) -> Result<i64> {
    entry.update(|info| info.read()).await
}

fn known_tags_iter<'a>(repo: &'a mut ScriptRepo) -> impl Iterator<Item = &'a Tag> {
    use std::collections::hash_map::Entry::*;

    let mut map: HashMap<&Tag, _> = Default::default();
    for script in repo.iter_mut(true) {
        let script = script.into_inner();
        let date = script.last_major_time();
        for tag in script.tags.iter() {
            match map.entry(tag) {
                Occupied(entry) => {
                    let date_mut = entry.into_mut();
                    *date_mut = std::cmp::max(date, *date_mut);
                }
                Vacant(entry) => {
                    entry.insert(date);
                }
            }
        }
    }
    let mut v: Vec<_> = map.into_iter().map(|(k, v)| (k, v)).collect();
    v.sort_by_key(|(_, v)| std::cmp::Reverse(*v));
    v.into_iter().map(|(k, _)| k)
}

async fn process_event_by_id(is_humble: bool, repo: RepoHolder, event_id: u64) -> Result {
    let (env, closer) = repo.env().await?;
    let event_id = event_id as i64;
    let res = if is_humble {
        env.historian.humble_args_by_id(event_id).await?
    } else {
        env.historian.ignore_args_by_id(event_id).await?
    };
    if let Some(res) = res {
        env.update_last_time_directly(res).await?;
    }
    closer.close(env).await;
    Ok(())
}

fn check_time_changed(entry: &RepoEntry<'_>, ignrore_res: &LastTimeRecord) -> bool {
    let s_exec_time = entry.exec_time.as_ref().map(|t| **t);
    let s_exec_done_time = entry.exec_done_time.as_ref().map(|t| **t);
    (s_exec_time, s_exec_done_time, entry.humble_time)
        != (
            ignrore_res.exec_time,
            ignrore_res.exec_done_time,
            ignrore_res.humble_time,
        )
}
