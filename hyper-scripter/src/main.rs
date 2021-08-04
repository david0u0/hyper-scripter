use chrono::Utc;
use hyper_scripter::args::{self, History, List, Root, Subs};
use hyper_scripter::config::{Config, NamedTagFilter};
use hyper_scripter::error::{Contextable, Error, RedundantOpt, Result};
use hyper_scripter::extract_msg::{extract_env_from_content, extract_help_from_content};
use hyper_scripter::list::{fmt_list, DisplayIdentStyle, DisplayStyle, ListOptions};
use hyper_scripter::query::{self, RangeQuery, ScriptQuery};
use hyper_scripter::script::ScriptName;
use hyper_scripter::script_repo::{RecentFilter, ScriptRepo};
use hyper_scripter::script_time::ScriptTime;
use hyper_scripter::tag::TagFilter;
use hyper_scripter::{
    path,
    util::{self, main_util, main_util::EditTagArgs},
};
use hyper_scripter_historian::Historian;

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
    if root.dump_args {
        let dumped = serde_json::to_string(&root)?;
        print!("{}", dumped);
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
    let conf = Config::get();
    let mut need_journal = main_util::need_write(root.subcmd.as_ref().unwrap());
    let (pool, init) = hyper_scripter::db::get_pool(&mut need_journal).await?;

    let recent = if root.timeless {
        None
    } else {
        root.recent.or(conf.recent).map(|recent| RecentFilter {
            recent,
            archaeology: root.archaeology,
        })
    };

    let historian = Historian::new(path::get_home().to_owned()).await?;
    let mut repo = ScriptRepo::new(pool, recent, historian.clone(), root.no_trace, need_journal)
        .await
        .context("讀取歷史記錄失敗")?;

    if init {
        log::info!("初次使用，載入好用工具和預執行腳本");
        main_util::load_utils(&mut repo).await?;
        main_util::prepare_pre_run()?;
        main_util::load_templates()?;
    }

    let explicit_filter = !root.filter.is_empty();
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
        Subs::LoadUtils => main_util::load_utils(&mut repo).await?,
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
                main_util::edit_or_create(edit_query, &mut repo, ty, edit_tags).await?;
            let prepare_resp = util::prepare_script(&path, &*entry, no_template, &content)?;
            if !fast {
                let cmd = util::create_concat_cmd(&conf.editor, &[&path]);
                let stat = util::run_cmd(cmd)?;
                if !stat.success() {
                    let code = stat.code().unwrap_or_default();
                    ret.errs.push(Error::EditorError(code, conf.editor.clone()));
                }
            }
            let should_keep = main_util::after_script(&mut entry, &path, &prepare_resp).await?;
            if !should_keep {
                let name = entry.name.clone();
                repo.remove(&name).await?
            }
        }
        Subs::Help { args } => {
            let script_query: ScriptQuery = args[0].parse()?;
            let entry = query::do_script_query_strict(&script_query, &mut repo).await?;
            log::info!("檢視用法： {:?}", entry.name);
            let script_path = path::open_script(&entry.name, &entry.ty, Some(true))?;
            let content = util::read_file(&script_path)?;

            let helps = extract_help_from_content(&content);
            for msg in helps {
                println!("{}", msg);
            }
            let envs = extract_env_from_content(&content);
            let mut first = true;
            for msg in envs {
                if first {
                    first = false;
                    println!("");
                }
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
            main_util::run_n_times(
                repeat,
                dummy,
                &mut entry,
                &args,
                historian,
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
            log::info!("打印 {:?}", entry.name);
            let script_path = path::open_script(&entry.name, &entry.ty, Some(true))?;
            let content = util::read_file(&script_path)?;
            print!("{}", content);
            entry.update(|info| info.read()).await?;
        }
        Subs::EnvHelp { script_query } => {
            let entry = query::do_script_query_strict(&script_query, &mut repo).await?;
            log::info!("打印 {:?} 的環境變數", entry.name);
            let script_path = path::open_script(&entry.name, &entry.ty, Some(true))?;
            let content = util::read_file(&script_path)?;
            let envs = extract_env_from_content(&content);
            for msg in envs {
                println!("{}", msg);
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
                    let res = main_util::mv(&mut entry, new_name, None, delete_tag.clone()).await;
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
            subcmd: History::RM { script, range },
        } => {
            let mut entry = query::do_script_query_strict(&script, &mut repo).await?;
            let res = match range {
                RangeQuery::Single(n) => historian.ignore_args(entry.id, n).await?,
                RangeQuery::Range { min, max } => {
                    historian.ignore_args_range(entry.id, min, max).await?
                }
            };
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
            subcmd: History::RMID { event_id },
        } => {
            let res = historian.ignore_args_by_id(event_id as i64).await?;
            if let Some(res) = res {
                let mut entry = repo.get_mut_by_id(res.script_id).ok_or_else(|| {
                    log::error!("史學家給的腳本 id 竟然在倉庫中找不到……");
                    Error::ScriptNotFound(res.script_id.to_string())
                })?;
                entry
                    .update(|info| {
                        info.exec_time = res.exec_time.map(|t| ScriptTime::new(t));
                        info.exec_done_time = res.exec_done_time.map(|t| ScriptTime::new(t));
                    })
                    .await?;
            }
        }
        Subs::History {
            subcmd: History::Amend { event_id, args },
        } => {
            let args = serde_json::to_string(&args)?;
            historian.amend_args_by_id(event_id as i64, &args).await?
        }
        Subs::History {
            subcmd: History::Tidy { queries },
        } => {
            for entry in query::do_list_query(&mut repo, &queries).await?.into_iter() {
                historian.tidy(entry.id).await?;
            }
        }
        Subs::History {
            subcmd: History::Neglect { queries },
        } => {
            for entry in query::do_list_query(&mut repo, &queries).await?.into_iter() {
                let id = entry.id;
                entry.get_env().handle_neglect(id).await?;
            }
        }
        Subs::History {
            subcmd:
                History::Show {
                    script,
                    limit,
                    with_name,
                    offset,
                },
        } => {
            let entry = query::do_script_query_strict(&script, &mut repo).await?;
            let args_list = historian.last_args_list(entry.id, limit, offset).await?;
            for args in args_list {
                log::debug!("嘗試打印參數 {}", args);
                let args: Vec<String> = serde_json::from_str(&args)?;
                if with_name {
                    print!("{}", entry.name.key());
                }
                for arg in args {
                    print!(" {}", util::to_display_args(arg)?);
                }
                println!();
            }
        }
        sub => unimplemented!("{:?}", sub),
    }
    Ok(ret)
}
