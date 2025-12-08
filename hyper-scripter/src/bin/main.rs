use futures::future::try_join_all;
use fxhash::{FxHashMap as HashMap, FxHashSet as HashSet};
use hyper_scripter::args::{
    self, ArgsResult, History, HistoryDisplay, List, Root, Subs, Tags, TagsSubs, Types, TypesSubs,
};
use hyper_scripter::config::{config_file, Config, NamedTagSelector};
use hyper_scripter::db;
use hyper_scripter::env_pair::EnvPair;
use hyper_scripter::error::{Contextable, DisplayError, Error, ExitCode, RedundantOpt, Result};
use hyper_scripter::extract_msg::{extract_env_from_content, extract_help_from_content};
use hyper_scripter::list::{fmt_list, DisplayStyle, ListOptions};
use hyper_scripter::my_env_logger;
use hyper_scripter::path;
use hyper_scripter::query::{self, EditQuery, ListQuery, ScriptOrDirQuery, ScriptQuery};
use hyper_scripter::script::{IntoScriptName, ScriptInfo, ScriptName};
use hyper_scripter::script_repo::{RepoEntry, ScriptRepo, Visibility};
use hyper_scripter::script_time::ScriptTime;
use hyper_scripter::tag::{Tag, TagSelector};
use hyper_scripter::to_display_args;
use hyper_scripter::util::{
    self, completion_util,
    holder::{RepoHolder, Resource},
    main_util::{self, EditTagArgs},
    print_iter,
};
use hyper_scripter_historian::{Historian, LastTimeRecord};

#[tokio::main]
async fn main() {
    my_env_logger::init();
    let mut errs = vec![];
    if let Err(err) = main_err_handle(&mut errs).await {
        errs.push(err);
    }
    let mut exit_code = ExitCode::default();
    for err in errs.iter() {
        exit_code.cmp_and_replace(err.code());
        eprint!("{}", err);
    }
    std::process::exit(exit_code.code());
}
async fn main_err_handle(errs: &mut Vec<Error>) -> Result {
    let args: Vec<_> = std::env::args().collect();
    let root = args::handle_args(args)?;
    let root = match root {
        ArgsResult::Normal(root) => root,
        ArgsResult::Shell(shell) => std::process::exit(util::run_shell(&shell)?),
        ArgsResult::Completion(comp) => {
            let mut repo = None;
            let res = completion_util::handle_completion(comp, &mut repo).await;
            if let Some(repo) = repo {
                repo.close().await;
            }
            res?;
            std::process::exit(0);
        }
        ArgsResult::Err(err) => {
            log::warn!("補捉到參數解析錯誤");
            err.print().unwrap();
            std::process::exit(1);
        }
    };
    if root.root_args.dump_args {
        let dumped = serde_json::to_string(&root)?;
        print!("{}", dumped);
        return Ok(());
    }

    root.set_home_unless_from_alias(true)?;

    if matches!(root.subcmd, Some(Subs::Migrate)) {
        db::do_migrate(db::get_file()).await?;
        Historian::do_migrate(path::get_home()).await?;
        return Ok(());
    }

    let mut resource = Resource::None;
    let mut ret = MainReturn { conf: None, errs };
    let res = main_inner(root, &mut resource, &mut ret).await;
    resource.close().await; // 不論如何都要關閉資料庫！
    res?;

    if let Some(conf) = ret.conf {
        log::info!("存入改變後的設定檔");
        conf.store()?;
    } else if Config::get().is_from_dafault() {
        log::info!("存入憑空產生的設定檔");
        Config::get().store()?;
    }
    Ok(())
}

struct MainReturn<'a> {
    conf: Option<Config>,
    /// 用來裝那種不會馬上造成中止的錯誤，例如 ScriptError
    errs: &'a mut Vec<Error>,
}

async fn main_inner(root: Root, resource: &mut Resource, ret: &mut MainReturn<'_>) -> Result {
    Config::set_runtime_conf(root.root_args.prompt_level);
    let explicit_select = !root.root_args.select.is_empty();

    let conf = Config::get();
    let need_journal = main_util::need_write(root.subcmd.as_ref().unwrap());

    let repo = RepoHolder {
        resource,
        need_journal,
        root_args: root.root_args,
    };

    macro_rules! conf_mut {
        () => {{
            ret.conf = Some(conf.clone());
            ret.conf.as_mut().unwrap()
        }};
    }

    match root.subcmd.unwrap() {
        Subs::LoadUtils => {
            let repo = repo.init().await?;
            main_util::load_utils(repo, None).await?;
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
                println!("{}\t{}", before, after);
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
            ..
        } => {
            log::info!("印出所有別名");
            for (before, alias) in conf.alias.iter() {
                let after = alias.after.join(" ");
                println!("{}\t{}", before, after);
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
            let repo = repo.init().await?;
            let edit_tags = {
                // TODO: 不要這麼愛 clone
                let mut innate_tags = conf.main_tag_selector.clone();
                if let Some(tags) = tags {
                    innate_tags.push(tags);
                    EditTagArgs {
                        explicit_select,
                        explicit_tag: true,
                        content: innate_tags,
                    }
                } else {
                    EditTagArgs {
                        explicit_select,
                        explicit_tag: false,
                        content: innate_tags,
                    }
                }
            };

            let (edit_res, create_res) =
                main_util::edit_or_create(edit_query, repo, ty, edit_tags).await?;

            let mut prepare_vec = vec![];
            for mut entry in edit_res.existing.into_iter() {
                create_read_event(&mut entry).await?;
                let p = path::open_script(&entry.name, &entry.ty, Some(true))
                    .context(format!("打開命名腳本失敗：{:?}", entry.name))?;
                let prepare_resp = util::prepare_script(&p, &*entry, None, &content)?;
                prepare_vec.push((entry.id, p, prepare_resp));
            }
            if let Some(create_res) = create_res {
                for (name, path) in create_res.to_create.into_iter() {
                    log::info!("創造 {:?}", name);
                    let template = if no_template {
                        None
                    } else {
                        // NOTE: 計算 `path` 時早已檢查過腳本類型，這裡直接不檢查了
                        Some(util::get_or_create_template(&create_res.ty, true, true)?)
                    };
                    let entry = repo
                        .entry(&name)
                        .or_insert(
                            ScriptInfo::builder(
                                0,
                                0,
                                name,
                                create_res.ty.ty.clone(),
                                create_res.tags.clone().into_iter(),
                            )
                            .build(),
                        )
                        .await?;
                    let prepare_resp = util::prepare_script(&path, &*entry, template, &content)?;
                    prepare_vec.push((entry.id, path, prepare_resp));
                }
            }

            if !fast {
                let res = util::open_editor(prepare_vec.iter().map(|(_, p, _)| p.as_ref()));
                if let Err(err) = res {
                    ret.errs.push(err);
                }
            }

            for (id, path, prepare_resp) in prepare_vec.iter() {
                let prepare_resp = if fast { None } else { Some(*prepare_resp) };
                let mut entry = repo.get_mut_by_id(*id).unwrap();
                let after_res = main_util::after_script(&mut entry, &path, prepare_resp).await;
                match after_res {
                    Ok(_) => (),
                    Err(err @ Error::EmptyCreate) => {
                        util::remove(&path)?;
                        let id = entry.id;
                        repo.remove(id).await?;
                        ret.errs.push(err);
                    }
                    Err(err) => return Err(err),
                }
            }
        }
        Subs::Help { args } => {
            let repo = repo.init().await?;
            let script_query: ScriptQuery =
                args[0].parse().map_err(|e: DisplayError| e.into_err())?;
            let mut entry = query::do_script_query_strict(&script_query, repo).await?;
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
        }
        Subs::Config => {
            let file = config_file(path::get_home());
            print!("{}", file.to_string_lossy());
        }
        Subs::Run {
            no_caution,
            script_query,
            dummy,
            args,
            previous,
            error_no_previous,
            repeat,
            dir,
        } => {
            let repo = repo.init().await?;
            let dir = util::option_map_res(dir, |d| path::normalize_path(d))?;
            let mut entry = query::do_script_query_strict(&script_query, repo).await?;
            main_util::run_n_times(
                repeat.unwrap_or(1),
                dummy,
                &mut entry,
                args,
                &mut ret.errs,
                previous,
                error_no_previous,
                !no_caution,
                dir,
            )
            .await?;
        }
        Subs::Which { queries } => {
            let repo = repo.init().await?;
            let home = path::get_home();
            let mut scripts = query::do_list_query(repo, queries).await?;
            scripts.sort_by_key(|s| std::cmp::Reverse(s.last_time()));
            for entry in scripts.into_iter() {
                log::info!("定位 {:?}", entry.name);
                // NOTE: 不檢查存在與否
                let p = home.join(entry.file_path_fallback());
                println!("{}", p.to_string_lossy());
            }
        }
        Subs::Cat { queries, with } => {
            let repo = repo.init().await?;
            let mut scripts = query::do_list_query(repo, queries).await?;
            scripts.sort_by_key(|s| std::cmp::Reverse(s.last_time()));
            let mut cmd = with.map(|w| util::create_concat_cmd_shlex(&w, Option::<&str>::None));
            for entry in scripts.iter_mut() {
                log::info!("打印 {:?}", entry.name);
                let script_path = path::open_script(&entry.name, &entry.ty, Some(true))?;
                if let Some(cmd) = cmd.as_mut() {
                    cmd.arg(script_path);
                } else {
                    let content = util::read_file(&script_path)?;
                    print!("{}", content);
                }
                create_read_event(entry).await?;
            }
            if let Some(cmd) = cmd {
                let code = util::run_cmd(cmd)?;
                if let Some(code) = code {
                    return Err(Error::ScriptError(code));
                }
            }
        }
        Subs::Types(Types {
            subcmd: Some(TypesSubs::LS { no_sub }),
        }) => {
            let mut first = true;
            for ty in conf.types.keys() {
                if !first {
                    print!(" ");
                }
                first = false;
                print!("{}", ty);
                if !no_sub {
                    let subs = path::get_sub_types(ty)?;
                    for sub in subs.into_iter() {
                        print!(" {}/{}", ty, sub);
                    }
                }
            }
        }
        Subs::Types(Types {
            subcmd: Some(TypesSubs::Template { ty, edit }),
        }) => {
            if edit {
                let (tmpl_path, _) = util::get_or_create_template_path(&ty, false, false)?;
                util::open_editor([tmpl_path.as_ref()])?;
            } else {
                let template = util::get_or_create_template(&ty, false, false)?;
                println!("{}", template);
            }
        }
        Subs::Recent { recent_filter } => {
            let conf = conf_mut!();
            if let Some(recent) = recent_filter {
                conf.recent = recent;
            } else {
                println!("{}", conf.recent);
            }
        }
        Subs::LS(List {
            long,
            grouping,
            limit,
            queries,
            plain,
            format,
        }) => {
            let display_style = if long {
                DisplayStyle::Long(())
            } else {
                DisplayStyle::Short(format, ())
            };
            let opt = ListOptions {
                grouping: grouping.into(),
                plain,
                limit,
                display_style,
            };
            let stdout = std::io::stdout();
            let repo = repo.init().await?;
            fmt_list(&mut stdout.lock(), repo, opt, queries).await?;
        }
        Subs::RM { queries, purge } => {
            let repo = repo.init().await?;
            let delete_tag: Option<TagSelector> = Some("+remove".parse().unwrap());
            let mut to_purge = vec![]; // (Option<path>, id)
            for mut entry in query::do_list_query(repo, queries).await?.into_iter() {
                log::info!("刪除 {:?}", *entry);
                let try_open_res = path::open_script(&entry.name, &entry.ty, Some(true));
                if purge || entry.name.is_anonymous() {
                    log::debug!("真的刪除腳本！");
                    let p = match try_open_res {
                        Ok(p) => Some(p),
                        Err(e) => {
                            log::warn!("試開腳本時出錯：{}", e);
                            None
                        }
                    };
                    to_purge.push((p, entry.id));
                } else {
                    log::debug!("不要真的刪除腳本，改用標籤隱藏之：{:?}", entry.name);
                    match try_open_res {
                        Err(Error::PathNotFound(_)) => {
                            log::warn!("{:?} 實體不存在，消滅之", entry.name);
                            to_purge.push((None, entry.id));
                        }
                        Err(e) => {
                            log::warn!("試開腳本時出錯：{}", e); // e.g. unknown type
                        }
                        _ => (),
                    }
                    main_util::mv(&mut entry, None, None, delete_tag.clone()).await?;
                }
            }
            for (p, id) in to_purge.into_iter() {
                repo.remove(id).await?;
                if let Some(p) = p {
                    if let Err(e) = util::remove(&p) {
                        log::warn!("刪除腳本實體遭遇錯誤：{}", e);
                    }
                }
            }
        }
        Subs::CP { origin, new, tags } => {
            let repo = repo.init().await?;
            let cp_pairs = create_dir_pair(repo, origin, new).await?;
            for (og_name, new_name) in cp_pairs.into_iter() {
                // TODO: 用 id 之類的加速？
                let entry = repo.get_mut(&og_name, Visibility::All).unwrap();
                let mut new_info = entry.cp(new_name);
                let og_path = path::open_script(&og_name, &entry.ty, Some(true))?;
                let new_path = path::open_script(&new_info.name, &entry.ty, Some(false))?;
                util::cp(&og_path, &new_path)?;

                if let Some(tags) = &tags {
                    new_info.append_tags(tags.clone());
                }
                repo.entry(&new_info.name).or_insert(new_info).await?;
            }
        }
        Subs::MV {
            origin,
            new,
            tags,
            ty,
        } => {
            let repo = repo.init().await?;
            if let Some(new) = new {
                let mv_pairs = create_dir_pair(repo, origin, new).await?;
                for (og_name, new_name) in mv_pairs.into_iter() {
                    // TODO: 用 id 之類的加速？
                    let mut script = repo.get_mut(&og_name, Visibility::All).unwrap();
                    main_util::mv(&mut script, Some(new_name), ty.clone(), tags.clone()).await?;
                }
            } else {
                let mut scripts = query::do_list_query(repo, std::iter::once(origin)).await?;
                for entry in scripts.iter_mut() {
                    main_util::mv(entry, None, ty.clone(), tags.clone()).await?;
                }
            }
        }
        Subs::Tags(Tags {
            subcmd: Some(TagsSubs::Unset { name }),
        }) => {
            let conf = conf_mut!();
            let pos = conf
                .tag_selectors
                .iter()
                .position(|f| f.name == name)
                .ok_or_else(|| {
                    log::error!("試著刪除不存在的選擇器 {:?}", name);
                    Error::TagSelectorNotFound(name)
                })?;
            conf.tag_selectors.remove(pos);
        }
        Subs::Tags(Tags {
            subcmd: Some(TagsSubs::Toggle { names }),
        }) => {
            let conf = conf_mut!();
            for name in names.into_iter() {
                let selector = conf
                    .tag_selectors
                    .iter_mut()
                    .find(|f| f.name == name)
                    .ok_or_else(|| {
                        log::error!("試著切換不存在的選擇器 {:?}", name);
                        Error::TagSelectorNotFound(name)
                    })?;
                selector.inactivated = !selector.inactivated;
            }
        }
        Subs::Tags(Tags {
            subcmd:
                Some(TagsSubs::LS {
                    named: true,
                    known: false,
                }),
        }) => {
            print_iter(conf.tag_selectors.iter().map(|f| &f.name), " ");
        }
        Subs::Tags(Tags {
            subcmd:
                Some(TagsSubs::LS {
                    named: false,
                    known,
                }),
        }) => {
            let repo = repo.init().await?;
            if known {
                print_iter(known_tags_iter(repo), " ");
            } else {
                print!("known tags:\n  ");
                print_iter(known_tags_iter(repo), " ");
                println!("");

                println!("caution tags:");
                println!("  {}", conf.caution_tags);

                println!("tag selector:");
                for selector in conf.tag_selectors.iter() {
                    let content = &selector.content;
                    print!("  {} = {}", selector.name, content);
                    if selector.inactivated {
                        print!(" (inactivated)")
                    }
                    println!()
                }
                println!("main tag selector:");
                print!("  {}", conf.main_tag_selector);
                println!();
            }
        }
        Subs::Tags(Tags {
            subcmd: Some(TagsSubs::Set { content, name }),
        }) => {
            let conf = conf_mut!();
            if let Some(name) = name {
                log::debug!("處理選擇器 {:?}", name);
                let tag_selector: &mut Vec<NamedTagSelector> = &mut conf.tag_selectors;
                if let Some(existing_selector) = tag_selector.iter_mut().find(|f| f.name == name) {
                    log::info!("修改選擇器 {} {}", name, content);
                    existing_selector.content = content;
                } else {
                    log::info!("新增選擇器 {} {}", name, content);
                    tag_selector.push(NamedTagSelector {
                        content: content,
                        name,
                        inactivated: false,
                    });
                }
            } else {
                log::info!("加入主選擇器 {:?}", content);
                conf.main_tag_selector = content;
            }
        }
        Subs::History {
            subcmd:
                History::RM {
                    queries,
                    dir,
                    range,
                    no_humble,
                    display,
                },
        } => {
            let repo = repo.init().await?;
            let historian = repo.historian().clone();
            let mut scripts = query::do_list_query(repo, queries).await?;
            let ids: Vec<_> = scripts.iter().map(|s| s.id).collect();
            let dir = util::option_map_res(dir, |d| path::normalize_path(d))?;

            let res_vec = historian
                .ignore_args_range(
                    &ids,
                    dir.as_deref(),
                    no_humble,
                    display.show_env(),
                    display.show_args(),
                    range.get_min(),
                    range.get_max(),
                )
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
            subcmd:
                History::Amend {
                    event_id,
                    args,
                    env,
                    no_env,
                },
        } => match event_id.try_into() {
            Err(_) => log::info!("試圖修改零事件，什麼都不做"),
            Ok(event_id) => {
                let historian = repo.historian().await?;
                let args = serde_json::to_string(&args)?;
                let env = if no_env || !env.is_empty() {
                    Some(serde_json::to_string(&env)?)
                } else {
                    None
                };
                historian
                    .amend_args_by_id(event_id, &args, env.as_deref())
                    .await?;
            }
        },
        Subs::History {
            subcmd: History::Tidy,
        } => {
            let repo = repo.init().await?;
            let historian = repo.historian().clone();

            let id_vec: Vec<_> = repo.iter_mut(Visibility::All).map(|e| e.id).collect();
            let tidy_fut = id_vec.iter().map(|id| historian.tidy(*id));
            try_join_all(tidy_fut).await?;
            historian.clear_except_script_ids(&id_vec).await?;
        }
        Subs::History {
            subcmd: History::Neglect { queries },
        } => {
            let repo = repo.init().await?;
            for entry in query::do_list_query(repo, queries).await?.into_iter() {
                let id = entry.id;
                entry.get_env().handle_neglect(id).await?;
            }
        }
        Subs::History {
            subcmd:
                History::Show {
                    queries,
                    limit,
                    with_name,
                    no_humble,
                    offset,
                    display,
                    dir,
                },
        } => {
            let repo = repo.init().await?;
            let historian = repo.historian().clone();
            let dir = util::option_map_res(dir, |d| path::normalize_path(d))?;
            let scripts = query::do_list_query(repo, queries).await?;
            let ids: Vec<_> = scripts.iter().map(|s| s.id).collect();

            enum ScriptGetter<'a> {
                Single(&'a ScriptInfo),
                Repo(&'a mut ScriptRepo),
            }
            impl<'a> ScriptGetter<'a> {
                fn get_by_id(repo: &mut ScriptRepo, id: i64) -> Result<&ScriptInfo> {
                    let entry = repo.get_mut_by_id(id).ok_or_else(|| {
                        log::error!("史學家給的腳本 id {} 竟然在倉庫中找不到……", id);
                        Error::ScriptNotFound(id.to_string())
                    })?;
                    Ok(entry.into_inner())
                }
                fn get<'b>(&'b mut self, id: i64) -> Result<&'b ScriptInfo> {
                    match self {
                        ScriptGetter::Single(info) => Ok(*info),
                        ScriptGetter::Repo(repo) => Self::get_by_id(repo, id),
                    }
                }
                fn new(ids: &[i64], repo: &'a mut ScriptRepo) -> Result<Self> {
                    Ok(if ids.len() == 1 {
                        ScriptGetter::Single(ScriptGetter::get_by_id(repo, ids[0])?)
                    } else {
                        ScriptGetter::Repo(repo)
                    })
                }
            }

            let mut script_getter = ScriptGetter::new(&ids, repo)?;
            let mut print_basic = |script_id: i64, args: Vec<String>| -> Result {
                if with_name {
                    let info = script_getter.get(script_id)?;
                    print!("{}", info.name.key());
                    if !args.is_empty() {
                        print!(" ");
                    }
                }
                print_iter(args.iter().map(|s| to_display_args(s)), " ");
                println!("");
                Ok(())
            };

            match display {
                HistoryDisplay::All => {
                    let args_list = historian
                        .previous_args_list_with_envs(
                            &ids,
                            limit,
                            offset,
                            no_humble,
                            dir.as_deref(),
                        )
                        .await?;
                    for (script_id, args, envs) in args_list {
                        log::debug!("嘗試打印參數 {} {} {}", script_id, args, envs);
                        let args: Vec<String> = serde_json::from_str(&args)?;
                        let envs: Vec<EnvPair> = serde_json::from_str(&envs)?;
                        print_basic(script_id, args)?;
                        for p in envs.into_iter() {
                            println!("  {}", p);
                        }
                    }
                }
                HistoryDisplay::Args => {
                    let args_list = historian
                        .previous_args_list(&ids, limit, offset, no_humble, dir.as_deref())
                        .await?;
                    for (script_id, args) in args_list {
                        log::debug!("嘗試打印參數 {} {}", script_id, args);
                        let args: Vec<String> = serde_json::from_str(&args)?;
                        print_basic(script_id, args)?;
                    }
                }
                HistoryDisplay::Env => {
                    let args_list = historian
                        .previous_args_list_only_envs(
                            &ids,
                            limit,
                            offset,
                            no_humble,
                            dir.as_deref(),
                        )
                        .await?;
                    for (script_id, envs) in args_list {
                        log::debug!("嘗試打印參數 {} {}", script_id, envs);
                        let envs: Vec<EnvPair> = serde_json::from_str(&envs)?;
                        print_basic(script_id, vec![])?;
                        for p in envs.into_iter() {
                            println!("  {}", p);
                        }
                    }
                }
            }
        }
        Subs::Top { id, queries, wait } => {
            let script_id_set: Option<HashSet<_>> = if queries.is_empty() {
                None
            } else {
                let repo = repo.init().await?;
                let scripts = query::do_list_query(repo, queries).await?;
                Some(scripts.iter().map(|e| e.id).collect())
            };

            let run_id_set: HashSet<_> = id.iter().collect();
            let processes = main_util::get_all_active_process_locks()?;

            for lock in processes.into_iter() {
                if !run_id_set.is_empty() {
                    if !run_id_set.contains(&(lock.get_run_id() as u64)) {
                        continue;
                    }
                }
                let info = &lock.process;
                if let Some(script_id_set) = &script_id_set {
                    if !script_id_set.contains(&info.script_id) {
                        continue;
                    }
                }
                if wait {
                    lock.wait_write()?;
                } else {
                    println!(
                        "{} {} {} {}",
                        info.pid,
                        lock.get_run_id(),
                        info.script_id,
                        info.file_content()
                    );
                }
            }
        }
        sub => unimplemented!("{:?}", sub),
    }
    Ok(())
}

async fn create_read_event(entry: &mut RepoEntry<'_>) -> Result<i64> {
    entry.update(|info| info.read()).await
}

fn known_tags_iter<'a>(repo: &'a mut ScriptRepo) -> impl Iterator<Item = &'a Tag> {
    use std::collections::hash_map::Entry::*;

    let mut map: HashMap<&Tag, _> = Default::default();
    for script in repo.iter_mut(Visibility::All) {
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

async fn process_event_by_id<'a>(is_humble: bool, repo: RepoHolder<'a>, event_id: u64) -> Result {
    match event_id.try_into() {
        Err(_) => log::info!("試圖處理零事件，什麼都不做"),
        Ok(event_id) => {
            let env = repo.env().await?;
            let res = if is_humble {
                env.historian.humble_args_by_id(event_id).await?
            } else {
                env.historian.ignore_args_by_id(event_id).await?
            };
            if let Some(res) = res {
                env.update_last_time_directly(res).await?;
            }
        }
    }
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

async fn create_dir_pair(
    repo: &mut ScriptRepo,
    og: ListQuery,
    new: EditQuery<ScriptOrDirQuery>,
) -> Result<Vec<(ScriptName, ScriptName)>> {
    let scripts = query::do_list_query(repo, std::iter::once(og)).await?;
    let is_dir = matches!(new, EditQuery::Query(ScriptOrDirQuery::Dir(_)));
    if scripts.len() > 1 && !is_dir {
        log::warn!("試圖把多個腳本移動成同一個");
        return Err(RedundantOpt::Scripts(
            scripts.iter().map(|s| s.name.key().to_string()).collect(),
        )
        .into());
    }
    let pairs_res: Result<Vec<_>> = scripts
        .into_iter()
        .map(|script| -> Result<_> {
            let new_name = match &new {
                EditQuery::NewAnonimous => path::new_anonymous_name(1, std::iter::empty())?
                    .next()
                    .unwrap(),
                EditQuery::Query(ScriptOrDirQuery::Script(new)) => new.clone(),
                EditQuery::Query(ScriptOrDirQuery::Dir(new)) => {
                    let new = new.clone();
                    new.join(&script.name).into_script_name()?
                }
            };
            Ok((script.name.clone(), new_name))
        })
        .collect();
    if let Ok(pairs) = &pairs_res {
        let mut dup_set = HashSet::default();
        for (_, new_name) in pairs.iter() {
            if dup_set.contains(new_name) || repo.get_mut(new_name, Visibility::All).is_some() {
                return Err(Error::ScriptExist(new_name.to_string()));
            }
            dup_set.insert(new_name);
        }
    }
    pairs_res
}
