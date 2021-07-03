use chrono::Utc;
use hyper_scripter::args::{self, print_help, History, List, Root, Subs};
use hyper_scripter::config::{Config, NamedTagFilter};
use hyper_scripter::error::{Contextable, Error, Result};
use hyper_scripter::extract_help;
use hyper_scripter::list::{fmt_list, DisplayIdentStyle, DisplayStyle, ListOptions};
use hyper_scripter::query::{self, ScriptQuery};
use hyper_scripter::script::ScriptName;
use hyper_scripter::script_repo::ScriptRepo;
use hyper_scripter::script_time::ScriptTime;
use hyper_scripter::tag::TagFilter;
use hyper_scripter::{
    path,
    util::{self, main_util::EditTagArgs},
};

#[tokio::main]
async fn main() {
    env_logger::init();
    let errs = match main_err_handle().await {
        Err(e) => vec![e],
        Ok(v) => v,
    };
    let mut exit_code = 0;
    for err in errs.iter() {
        match err {
            Error::ScriptError(c) | Error::PreRunError(c) => exit_code = *c,
            _ if exit_code == 0 => exit_code = 1,
            _ => (),
        }
        eprint!("{}", err);
    }
    std::process::exit(exit_code);
}
async fn main_err_handle() -> Result<Vec<Error>> {
    let args: Vec<_> = std::env::args().collect();
    let root = args::handle_args(&args)?;
    if root.dump_args {
        let dumped = serde_json::to_string(&root)?;
        print!("{}", dumped);
        return Ok(vec![]);
    }
    let res = main_inner(root).await?;
    if let Some(conf) = res.conf {
        conf.store()?;
    } else {
        Config::get()?.store()?;
    }
    Ok(res.errs)
}

struct MainReturn {
    conf: Option<Config>,
    errs: Vec<Error>,
}

async fn main_inner(root: Root) -> Result<MainReturn> {
    let conf = Config::get()?;
    let (pool, init) = hyper_scripter::db::get_pool().await?;
    let recent = if root.timeless {
        None
    } else {
        root.recent.or(conf.recent)
    };
    let mut repo = ScriptRepo::new(pool, recent)
        .await
        .context("讀取歷史記錄失敗")?;

    if init {
        log::info!("初次使用，載入好用工具和預執行腳本");
        util::main_util::load_utils(&mut repo).await?;
        util::main_util::prepare_pre_run()?;
    }

    let explicit_filter = !root.filter.is_empty();
    let historian = repo.historian().clone();
    let mut ret = MainReturn {
        conf: None,
        errs: vec![],
    };
    {
        let mut tag_group = conf.get_tag_filter_group(); // TODO: TagFilterGroup 可以多帶點 lifetime 減少複製
        for filter in root.filter.into_iter() {
            tag_group.push(filter);
        }
        repo.filter_by_tag(&tag_group);
    }

    macro_rules! conf_mut {
        () => {{
            ret.conf = Some(conf.clone());
            ret.conf.as_mut().unwrap()
        }};
    }

    match root.subcmd.unwrap() {
        Subs::LoadUtils => util::main_util::load_utils(&mut repo).await?,
        Subs::Alias {
            unset: false,
            before: Some(before),
            after,
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
            ..
        } => {
            log::info!("印出所有別名");
            for (before, alias) in conf.alias.iter() {
                let after = alias.after.join(" ");
                println!("{}=\"{}\"", before, after);
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
                util::main_util::edit_or_create(edit_query, &mut repo, ty, edit_tags).await?;
            let created = util::prepare_script(&path, &*entry, no_template, &content)?;
            if !fast {
                let cmd = util::create_concat_cmd(&conf.editor, &[&path]);
                let stat = util::run_cmd(cmd)?;
                log::debug!("編輯器返回：{:?}", stat);
            }
            let should_keep = util::after_script(&path, created)?;
            if should_keep {
                entry.update(|info| info.write()).await?;
            } else {
                let name = entry.name.clone();
                repo.remove(&name).await?
            }
        }
        Subs::Help { args } => {
            print_help(args.iter())?;
            let script_query: ScriptQuery = args[0].parse()?;
            let entry = query::do_script_query_strict(&script_query, &mut repo).await?;
            log::info!("檢視用法： {:?}", entry.name);

            extract_help!(helps, entry, true);
            for msg in helps {
                println!("{}", msg);
            }
        }
        Subs::Run {
            script_query,
            dummy,
            args,
            previous_args,
            repeat,
        } => {
            let mut entry = query::do_script_query_strict(&script_query, &mut repo).await?;
            util::main_util::run_n_times(
                repeat,
                dummy,
                &mut entry,
                &args,
                historian.clone(),
                &mut ret.errs,
                previous_args,
            )
            .await?;
        }
        Subs::Which { script_query } => {
            let entry = query::do_script_query_strict(&script_query, &mut repo).await?;
            log::info!("定位 {:?}", entry.name);
            let p = path::get_home().join(entry.file_path()?);
            println!("{}", p.to_string_lossy());
        }
        Subs::Cat { script_query } => {
            let mut entry = query::do_script_query_strict(&script_query, &mut repo).await?;
            let script_path = path::open_script(&entry.name, &entry.ty, Some(true))?;
            log::info!("打印 {:?}", entry.name);
            let content = util::read_file(&script_path)?;
            print!("{}", content);
            entry.update(|info| info.read()).await?;
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
                (true, false, false) => DisplayStyle::Long(()),
                _ => unreachable!(),
            };
            let opt = ListOptions {
                grouping: grouping.into(),
                plain,
                queries: &queries,
                display_style,
            };
            let stdout = std::io::stdout();
            fmt_list(&mut stdout.lock(), &mut repo, &opt).await?;
        }
        Subs::RM { queries, purge } => {
            let delete_tag: Option<TagFilter> = Some("+removed".parse().unwrap());
            let mut to_purge = vec![];
            for mut entry in query::do_list_query(&mut repo, &queries).await?.into_iter() {
                log::info!("刪除 {:?}", *entry);
                if purge {
                    log::debug!("真的刪除腳本！");
                    to_purge.push((entry.name.clone(), entry.ty.clone()));
                } else {
                    let time_str = Utc::now().format("%Y%m%d%H%M%S");
                    let new_name = util::change_name_only(&entry.name.to_string(), |name| {
                        format!("{}-{}", time_str, name)
                    });
                    log::debug!("不要真的刪除腳本，改用標籤隱藏之：{}", new_name);
                    let new_name = Some(ScriptName::Named(new_name));
                    let res =
                        util::main_util::mv(&mut entry, new_name, None, delete_tag.clone()).await;
                    match res {
                        Err(Error::PathNotFound(_)) => {
                            log::warn!("{:?} 實體不存在，消滅之", entry.name);
                            to_purge.push((entry.name.clone(), entry.ty.clone()));
                        }
                        Err(e) => return Err(e),
                        _ => (),
                    }
                }
            }
            for (name, ty) in to_purge.into_iter() {
                let p = path::open_script(&name, &ty, None)?;
                repo.remove(&name).await?;
                if let Err(e) = util::remove(&p) {
                    log::warn!("刪除腳本實體遭遇錯誤：{}", e);
                }
            }
        }
        Subs::CP { origin, new, tags } => {
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
        }
        Subs::MV {
            origin,
            new,
            tags,
            ty,
        } => {
            let new_name = match new {
                Some(name) => {
                    if repo.get_mut(&name, true).is_some() {
                        return Err(Error::ScriptExist(name.to_string()));
                    }
                    Some(name)
                }
                None => None,
            };
            let mut entry = query::do_script_query_strict(&origin, &mut repo).await?;
            util::main_util::mv(&mut entry, new_name, ty, tags).await?;
        }
        Subs::Tags {
            unset: Some(name), ..
        } => {
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
        Subs::Tags {
            switch_activated: Some(name),
            ..
        } => {
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
        Subs::Tags {
            tag_filter: None, ..
        } => {
            print!("known tags:\n  ");
            for t in repo.iter_known_tags() {
                print!("{} ", t);
            }
            println!();
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
        Subs::Tags {
            tag_filter: Some(filter),
            ..
        } => {
            let conf = conf_mut!();
            let content = filter.content;
            if let Some(name) = filter.name {
                log::debug!("處理篩選器 {:?}", name);
                let tag_filters: &mut Vec<NamedTagFilter> = &mut conf.tag_filters;
                if let Some(existing_filter) = tag_filters.iter_mut().find(|f| f.name == name) {
                    log::info!("修改篩選器 {} {}", name, content);
                    existing_filter.content = content;
                } else {
                    log::info!("新增篩選器 {} {}", name, content);
                    tag_filters.push(NamedTagFilter {
                        content,
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
            subcmd: History::RM { script, number },
        } => {
            let mut entry = query::do_script_query_strict(&script, &mut repo).await?;
            let res = historian.ignore_args(entry.id, number).await?;
            if let Some(res) = res {
                entry
                    .update(|info| {
                        info.exec_time = res.exec_time.map(|t| ScriptTime::new(t));
                        info.exec_done_time = res.exec_done_time.map(|t| ScriptTime::new(t));
                    })
                    .await?;
            }
        }
        Subs::History {
            subcmd: History::Tidy { queries },
        } => {
            for entry in query::do_list_query(&mut repo, &queries).await?.into_iter() {
                historian.tidy(entry.id).await?;
            }
        }
        Subs::History {
            subcmd:
                History::Show {
                    script,
                    limit,
                    offset,
                },
        } => {
            let entry = query::do_script_query_strict(&script, &mut repo).await?;
            let args_list = historian.last_args_list(entry.id, limit, offset).await?;
            for args in args_list {
                log::debug!("嘗試打印參數 {}", args);
                let args: Vec<String> = serde_json::from_str(&args)?;
                let mut first = true;
                for arg in args {
                    if !first {
                        print!(" ");
                    } else {
                        first = false;
                    }
                    print!("{}", util::to_display_args(arg)?);
                }
                println!();
            }
        }
        sub => unimplemented!("{:?}", sub),
    }
    Ok(ret)
}
