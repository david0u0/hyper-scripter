use chrono::Utc;
use hyper_scripter::args::{self, print_help, List, Root, Subs};
use hyper_scripter::config::{Config, NamedTagFilter};
use hyper_scripter::error::{Contextable, Error, Result};
use hyper_scripter::extract_help;
use hyper_scripter::list::{fmt_list, DisplayIdentStyle, DisplayStyle, ListOptions};
use hyper_scripter::query::{self, ScriptQuery};
use hyper_scripter::script::ScriptName;
use hyper_scripter::script_repo::ScriptRepo;
use hyper_scripter::tag::TagFilter;
use hyper_scripter::{
    path,
    util::{self, main_util::EditTagArgs},
};
use util::main_util::load_utils;

#[tokio::main]
async fn main() {
    env_logger::init();
    let errs = match main_err_handle().await {
        Err(e) => vec![e],
        Ok(v) => v,
    };
    for err in errs.iter() {
        eprint!("{}", err);
    }
    if errs.len() > 0 {
        std::process::exit(1);
    }
}
async fn main_err_handle() -> Result<Vec<Error>> {
    let root = args::handle_args()?;
    let mut conf = Config::get()?.clone();
    let res = main_inner(root, &mut conf).await?;
    conf.store()?;
    Ok(res)
}
async fn main_inner(root: Root, conf: &mut Config) -> Result<Vec<Error>> {
    let (pool, init) = hyper_scripter::db::get_pool().await?;
    let recent = if root.timeless {
        None
    } else {
        root.recent.or(conf.recent)
    };
    let mut repo = ScriptRepo::new(pool.clone(), recent)
        .await
        .context("讀取歷史記錄失敗")?;

    if init {
        log::info!("初次使用，載入好用工具");
        load_utils(&mut repo).await?;
    }

    let explicit_filter = root.filter.len() > 0;
    let historian = repo.historian().clone();
    let mut res = Vec::<Error>::new();
    {
        let mut tag_group = conf.get_tag_filter_group(); // TODO: TagFilterGroup 可以多帶點 lifetime 減少複製
        for filter in root.filter.into_iter() {
            tag_group.push(filter);
        }
        repo.filter_by_tag(&tag_group);
    }

    match root.subcmd.unwrap() {
        Subs::LoadUtils => load_utils(&mut repo).await?,
        Subs::Alias {
            unset: false,
            before: Some(before),
            after,
        } => {
            if after.len() > 0 {
                log::info!("設定別名 {} {:?}", before, after);
                conf.alias.insert(before, after.into());
            } else {
                log::info!("印出別名 {}", before);
                let after = conf
                    .alias
                    .get(&before)
                    .ok_or(Error::NoAlias(before.clone()))?
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
            category: ty,
            fast,
            tags,
            content,
            no_template,
        } => {
            // TODO: 這裡邏輯太複雜了，抽出來測試吧
            let edit_tags = {
                // TODO: 不要這麼愛 clone
                // TODO: 真的需要考慮到 root.filter 嗎？
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
            if content.is_some() {
                log::info!("帶內容編輯 {:?}", entry.name);
                if path.exists() {
                    log::error!("不允許帶內容編輯已存在的腳本");
                    return Err(Error::ScriptExist(entry.name.to_string()));
                }
            }
            let content = content.as_ref().map(|s| s.as_str());
            let created = util::prepare_script(&path, &*entry, no_template, content)?;
            if !fast {
                let cmd = util::create_cmd("vim", &[&path]);
                let stat = util::run_cmd("vim", cmd)?;
                log::debug!("編輯器返回：{:?}", stat);
            }
            let exist = util::after_script(&path, created)?;
            if exist {
                entry.update(|info| info.write()).await?;
            } else {
                let name = entry.name.clone();
                repo.remove(&name).await?
            }
        }
        Subs::Help { args } => {
            print_help(args.iter())?;
            let script_query: ScriptQuery = args[0].parse()?;
            let entry =
                query::do_script_query_strict_with_missing(&script_query, &mut repo).await?;
            log::info!("檢視用法： {:?}", entry.name);

            extract_help!(helps, entry, true);
            for msg in helps {
                println!("{}", msg);
            }
        }
        Subs::Run {
            script_query,
            args,
            previous_args,
            repeat,
        } => {
            util::main_util::run_n_times(
                repeat,
                &script_query,
                &mut repo,
                &args,
                historian.clone(),
                &mut res,
                previous_args,
            )
            .await?;
        }
        Subs::Which { script_query } => {
            let entry =
                query::do_script_query_strict_with_missing(&script_query, &mut repo).await?;
            log::info!("定位 {:?}", entry.name);
            let p = path::get_home().join(entry.file_path()?);
            println!("{}", p.to_string_lossy());
        }
        Subs::Cat { script_query } => {
            let mut entry =
                query::do_script_query_strict_with_missing(&script_query, &mut repo).await?;
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
        Subs::CP { origin, new } => {
            if repo.get_mut(&new, true).is_some() {
                return Err(Error::ScriptExist(new.to_string()));
            }
            let entry = query::do_script_query_strict_with_missing(&origin, &mut repo).await?;
            let og_script = path::open_script(&entry.name, &entry.ty, Some(true))?;
            let new_script = path::open_script(&new, &entry.ty, Some(false))?;
            util::cp(&og_script, &new_script)?;
            let new_info = entry.cp(new.clone());
            repo.entry(&new).or_insert(new_info).await?;
        }
        Subs::MV {
            origin,
            new,
            tags,
            category: ty,
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
            let mut entry = query::do_script_query_strict_with_missing(&origin, &mut repo).await?;
            util::main_util::mv(&mut entry, new_name, ty, tags).await?;
        }
        Subs::Tags { tag_filter } => {
            if let Some(filter) = tag_filter {
                if let Some(name) = filter.name {
                    log::debug!("處理篩選器 {:?}", name);
                    let is_empty = filter.content.is_empty();
                    // 順便在這個變數中帶上「是否為既有篩選器」的訊息。若找到了既有的篩器，就把 content 的值搶過來
                    let mut content = Some(filter.content);
                    let tag_filters: &mut Vec<NamedTagFilter> = &mut conf.tag_filters;
                    if let Some((i, existing_filter)) = tag_filters
                        .iter_mut()
                        .enumerate()
                        .find(|(_, f)| f.name == name)
                    {
                        let content = content.take().unwrap();
                        if is_empty {
                            log::info!("刪除篩選器 {} {}", name, content);
                            tag_filters.remove(i);
                        } else {
                            log::info!("修改篩選器 {} {}", name, content);
                            existing_filter.content = content;
                        }
                    }

                    // 若 content == None 代表該值已經用在既有篩選器上了，底下就不用做
                    if let Some(content) = content {
                        if !is_empty {
                            log::info!("新增篩選器 {} {}", name, content);
                            tag_filters.push(NamedTagFilter { content, name });
                        } else {
                            log::error!("試著刪除不存在的篩選器 {:?}", name); // TODO: 應該報錯上去？
                        }
                    }
                } else {
                    log::info!("加入主篩選器 {:?}", filter);
                    conf.main_tag_filter = filter.content;
                }
            } else {
                print!("known tags:\n  ");
                for t in repo.iter_known_tags() {
                    print!("{} ", t);
                }
                println!("");
                println!("tag filters:");
                for filter in conf.tag_filters.iter() {
                    let content = &filter.content;
                    print!("  {} = [{}]", filter.name, content);
                    if content.mandatory {
                        print!(" (mandatory)")
                    }
                    println!("")
                }
                println!("main tag filter:");
                print!("  [{}]", conf.main_tag_filter);
                if conf.main_tag_filter.mandatory {
                    print!(" (mandatory)")
                }
                println!("");
            }
        }
        sub @ _ => unimplemented!("{:?}", sub),
    }
    Ok(res)
}
