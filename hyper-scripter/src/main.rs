use chrono::Utc;
use hyper_scripter::config::{Config, NamedTagFilter};
use hyper_scripter::error::{Contextable, Error, Result};
use hyper_scripter::list::{fmt_list, DisplayScriptIdent, DisplayStyle, ListOptions};
use hyper_scripter::query::{self, EditQuery, FilterQuery, ListQuery, ScriptQuery};
use hyper_scripter::script::{AsScriptName, ScriptInfo, ScriptName};
use hyper_scripter::script_repo::{ScriptRepo, ScriptRepoEntry};
use hyper_scripter::script_type::ScriptType;
use hyper_scripter::tag::{Tag, TagControlFlow, TagFilter, TagFilterGroup};
use hyper_scripter::{path, util};
use hyper_scripter_historian::{Event, EventData};
use std::borrow::Cow;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::clap::AppSettings::{
    self, AllowLeadingHyphen, DisableHelpFlags, DisableHelpSubcommand, DisableVersion,
    TrailingVarArg,
};
use structopt::StructOpt;

const NO_FLAG_SETTINGS: &[AppSettings] = &[
    AllowLeadingHyphen,
    DisableHelpFlags,
    TrailingVarArg,
    DisableHelpSubcommand,
    DisableVersion,
];

#[derive(StructOpt, Debug)]
#[structopt(setting = AllowLeadingHyphen)]
struct Root {
    #[structopt(short = "p", long, help = "Path to hyper script root")]
    hs_path: Option<String>,
    #[structopt(
        short,
        long,
        global = true,
        parse(try_from_str),
        help = "Filter by tags, e.g. `all,^mytag`"
    )]
    filter: Option<TagControlFlow>,
    #[structopt(short, long, global = true, help = "Shorthand for `-f=all,^removed`")]
    all: bool,
    #[structopt(long, global = true, help = "Show scripts within recent days.")]
    recent: Option<u32>,
    #[structopt(
        long,
        global = true,
        help = "Show scripts of all time.",
        conflicts_with = "recent"
    )]
    timeless: bool,
    #[structopt(subcommand)]
    subcmd: Option<Subs>,
}
#[derive(StructOpt, Debug)]
enum Subs {
    #[structopt(external_subcommand)]
    Other(Vec<String>),
    #[structopt(setting = AppSettings::Hidden)]
    LoadUtils,
    #[structopt(about = "Edit hyper script", alias = "e")]
    Edit {
        #[structopt(
            long,
            short,
            parse(try_from_str),
            help = "Category of the script, e.g. `sh`"
        )]
        category: Option<ScriptType>,
        #[structopt(long, short)]
        no_template: bool,
        #[structopt(parse(try_from_str), default_value = ".")]
        edit_query: EditQuery,
        content: Option<String>,
        #[structopt(
            long,
            requires("content"),
            help = "create script without invoking the editor"
        )]
        fast: bool,
    },
    #[structopt(about = "Run the script", settings = NO_FLAG_SETTINGS)]
    Run {
        #[structopt(default_value = "-", parse(try_from_str))]
        script_query: ScriptQuery,
        #[structopt(help = "Command line args to pass to the script")]
        args: Vec<String>,
    },
    #[structopt(about = "Execute the script query and get the exact file")]
    Which {
        #[structopt(default_value = "-", parse(try_from_str))]
        script_query: ScriptQuery,
    },
    #[structopt(about = "Print the script to standard output")]
    Cat {
        #[structopt(default_value = "-", parse(try_from_str))]
        script_query: ScriptQuery,
    },
    #[structopt(about = "Remove the script")]
    RM {
        #[structopt(parse(try_from_str), required = true, min_values = 1)]
        queries: Vec<ListQuery>,
        #[structopt(
            long,
            help = "Actually remove scripts, rather than hiding them with tag."
        )]
        purge: bool,
    },
    #[structopt(about = "List hyper scripts", alias = "l")]
    LS(List),
    #[structopt(about = "Copy the script to another one")]
    CP {
        #[structopt(parse(try_from_str))]
        origin: ScriptQuery,
        new: String,
    },
    #[structopt(about = "Move the script to another one")]
    MV {
        #[structopt(
            long,
            short,
            parse(try_from_str),
            help = "Category of the script, e.g. `sh`"
        )]
        category: Option<ScriptType>,
        #[structopt(short, long)]
        tags: Option<TagControlFlow>,
        #[structopt(parse(try_from_str))]
        origin: ScriptQuery,
        new: Option<String>,
    },
    #[structopt(
        about = "Manage script tags. If a tag filter is given, set it as default, otherwise show tag information."
    )]
    Tags {
        #[structopt(parse(try_from_str))]
        tag_filter: Option<FilterQuery>,
        #[structopt(long, short, help = "Set the filter to obligation")]
        obligation: bool, // FIXME: 這邊下 requires 不知為何會炸掉 clap
    },
}

#[derive(StructOpt, Debug)]
struct List {
    // TODO: 滿滿的其它排序/篩選選項
    #[structopt(short, long, help = "Show verbose information.")]
    long: bool,
    #[structopt(long, help = "Don't group by tags.")]
    no_grouping: bool,
    #[structopt(long, help = "No color and other decoration.")]
    plain: bool,
    #[structopt(long, help = "Show file path to the script.", conflicts_with_all = &["name", "long"])]
    file: bool,
    #[structopt(long, help = "Show only name of the script.", conflicts_with_all = &["file", "long"])]
    name: bool,
    #[structopt(parse(try_from_str))]
    queries: Vec<ListQuery>,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let errs = match main_err_handle().await {
        Err(e) => vec![e],
        Ok(v) => v,
    };
    for err in errs.iter() {
        eprintln!("{}", err);
    }
    if errs.len() > 0 {
        std::process::exit(1);
    }
}
async fn main_err_handle() -> Result<Vec<Error>> {
    let mut root = Root::from_args();
    log::debug!("命令行物件：{:?}", root);
    match &root.hs_path {
        Some(hs_path) => path::set_path(hs_path)?,
        None => path::set_path_from_sys()?,
    }
    let mut conf = Config::get()?.clone();

    match root.subcmd {
        None => {
            root.subcmd = Some(Subs::Edit {
                edit_query: EditQuery::Query(ScriptQuery::Prev(1)),
                category: None,
                content: None,
                fast: false,
                no_template: false,
            });
        }
        Some(Subs::Other(args)) => {
            log::info!("執行模式");
            let run = Subs::Run {
                script_query: FromStr::from_str(&args[0])?,
                args: args[1..args.len()].iter().map(|s| s.clone()).collect(),
            };
            root.subcmd = Some(run);
        }
        _ => (),
    }
    let res = main_inner(&root, &mut conf).await?;
    conf.store()?;
    Ok(res)
}
async fn main_inner(root: &Root, conf: &mut Config) -> Result<Vec<Error>> {
    let pool = hyper_scripter::db::get_pool().await?;
    let recent = if root.timeless {
        None
    } else {
        root.recent.or(conf.recent)
    };
    let mut repo = ScriptRepo::new(pool.clone(), recent)
        .await
        .context("讀取歷史記錄失敗")?;
    let historian = repo.historian().clone();
    let mut res = Vec::<Error>::new();
    {
        let tag_group: TagFilterGroup = if root.all {
            TagFilter::from_str("all,^removed").unwrap().into()
        } else {
            let mut group = conf.get_tag_filter_group();
            if let Some(flow) = root.filter.clone() {
                group.push(flow.into());
            }
            group
        };
        repo.filter_by_tag(&tag_group);
    }

    match root.subcmd.as_ref().unwrap() {
        Subs::LoadUtils => {
            let utils = hyper_scripter_util::get_all();
            for u in utils.into_iter() {
                log::info!("載入小工具 {:?}", u);
                let name = u.name.as_script_name()?;
                let ty = ScriptType::from_str(u.category)?;
                let tags: Vec<Tag> = if u.is_hidden {
                    vec![
                        Tag::from_str("util").unwrap(),
                        Tag::from_str("hide").unwrap(),
                    ]
                } else {
                    vec![Tag::from_str("util").unwrap()]
                };
                let p = path::open_script(&name, &ty, false)?;
                if p.exists() {
                    log::warn!("已存在的工具 {:?}，跳過", name);
                    continue;
                }
                let entry = repo
                    .upsert(&name, || {
                        ScriptInfo::builder(0, name.clone(), ty, tags.into_iter()).build()
                    })
                    .await?;
                util::prepare_script(&p, *entry, true, Some(u.content))?;
            }
        }
        Subs::Edit {
            edit_query,
            category: ty,
            fast,
            content,
            no_template,
        } => {
            let edit_tags = match root.filter.clone() {
                None => conf.main_tag_filter.clone().filter,
                Some(tags) => {
                    if tags.append {
                        let mut main_tags = conf.main_tag_filter.clone().filter;
                        main_tags.push(tags);
                        main_tags
                    } else {
                        tags
                    }
                }
            };
            let (path, mut entry) =
                edit_or_create(edit_query, &mut repo, ty.clone(), edit_tags).await?;
            if content.is_some() {
                log::info!("帶內容編輯 {:?}", entry.name);
                if path.exists() {
                    log::error!("不允許帶內容編輯已存在的腳本");
                    return Err(Error::ScriptExist(entry.name.to_string()));
                }
            }
            let content = content.as_ref().map(|s| s.as_str());
            let created = util::prepare_script(&path, &*entry, *no_template, content)?;
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
        Subs::Run { script_query, args } => {
            let mut entry = query::do_script_query_strict(script_query, &mut repo)?;
            log::info!("執行 {:?}", entry.name);
            {
                let exe = std::env::current_exe()?;
                let exe = exe.to_string_lossy();
                log::debug!("將 hs 執行檔的確切位置 {} 記錄起來", exe);
                util::write_file(&path::get_path().join(path::HS_EXECUTABLE_INFO_PATH), &exe)?;
            }
            let script_path = path::open_script(&entry.name, &entry.ty, true)?;
            let content = util::read_file(&script_path)?;
            entry.update(|info| info.exec(content)).await?;
            let ret_code: i32;
            let run_res = util::run(
                &script_path,
                &*entry,
                &args,
                entry.exec_time.as_ref().unwrap().data().unwrap(),
            );
            match run_res {
                Err(Error::ScriptError(code)) => {
                    ret_code = code;
                    res.push(run_res.unwrap_err());
                }
                Err(e) => return Err(e),
                Ok(_) => ret_code = 0,
            }
            let res = historian
                .record(Event {
                    data: EventData::ExecDone(ret_code),
                    script_id: entry.id,
                })
                .await;
            match &res {
                Ok(_) => (),
                Err(sqlx::error::Error::Database(err)) => {
                    if err.code().as_ref().map(|s| s.as_ref()) == Some("517") {
                        log::warn!("資料庫最後被鎖住了！ {:?}", err);
                    } else {
                        res?;
                    }
                }
                Err(_) => res?,
            }
        }
        Subs::Which { script_query } => {
            let entry = query::do_script_query_strict(script_query, &mut repo)?;
            log::info!("定位 {:?}", entry.name);
            let p = path::get_path().join(entry.file_path()?);
            println!("{}", p.to_string_lossy());
        }
        Subs::Cat { script_query } => {
            let mut entry = query::do_script_query_strict(script_query, &mut repo)?;
            let script_path = path::open_script(&entry.name, &entry.ty, true)?;
            log::info!("打印 {:?}", entry.name);
            let content = util::read_file(&script_path)?;
            println!("{}", content);
            entry.update(|info| info.read()).await?;
        }
        Subs::LS(List {
            long,
            no_grouping,
            queries,
            plain,
            name,
            file,
        }) => {
            let display_style = match (long, file, name) {
                (false, true, false) => DisplayStyle::Short(DisplayScriptIdent::File),
                (false, false, true) => DisplayStyle::Short(DisplayScriptIdent::Name),
                (false, false, false) => DisplayStyle::Short(DisplayScriptIdent::Normal),
                (true, false, false) => DisplayStyle::Long,
                _ => unreachable!(),
            };
            let opt = ListOptions {
                no_grouping: *no_grouping,
                plain: *plain,
                queries,
                display_style,
            };
            let stdout = std::io::stdout();
            fmt_list(&mut stdout.lock(), &mut repo, &opt)?;
        }
        Subs::RM { queries, purge } => {
            let delete_tag: Option<TagControlFlow> = Some(FromStr::from_str("+removed").unwrap());
            let mut to_purge = vec![];
            for entry in query::do_list_query(&mut repo, queries)?.into_iter() {
                log::info!("刪除 {:?}", *entry);
                if *purge {
                    log::debug!("真的刪除腳本！");
                    to_purge.push((entry.name.clone().into_static(), entry.ty.clone()));
                } else {
                    log::debug!("不要真的刪除腳本，改用標籤隱藏之");
                    let time_str = Utc::now().format("%Y%m%d%H%M%S");
                    let new_name = util::change_name_only(&entry.name.to_string(), |name| {
                        format!("{}-{}", time_str, name)
                    });
                    let new_name = Some(ScriptName::Named(Cow::Owned(new_name)));
                    mv(entry, new_name, None, &delete_tag).await?;
                }
            }
            for (name, ty) in to_purge.into_iter() {
                let p = path::open_script(&name, &ty, false)?;
                repo.remove(&name).await?;
                util::remove(&p)?;
            }
        }
        Subs::CP { origin, new } => {
            let h = query::do_script_query_strict(origin, &mut repo)?;
            let new_name = new.as_script_name()?;
            let og_script = path::open_script(&h.name, &h.ty, true)?;
            let new_script = path::open_script(&new_name, &h.ty, false)?;
            if new_script.exists() {
                return Err(Error::ScriptExist(new.clone()));
            }
            util::cp(&og_script, &new_script)?;
            let new_info = h.cp(new_name.clone());
            repo.upsert(&new_name, || new_info).await?;
        }
        Subs::MV {
            origin,
            new,
            tags,
            category: ty,
        } => {
            let new_name = match new {
                Some(s) => Some(s.as_script_name()?),
                None => None,
            };
            let entry = query::do_script_query_strict(origin, &mut repo)?;
            mv(entry, new_name, ty.as_ref(), tags).await?;
        }
        Subs::Tags {
            tag_filter,
            obligation,
        } => {
            if let Some(filter) = tag_filter {
                if let Some(name) = &filter.name {
                    log::debug!("處理篩選器 {:?}", filter);
                    let mut found = false;
                    let tag_filters = &mut conf.tag_filters;
                    for (i, f) in tag_filters.iter_mut().enumerate() {
                        if &f.name == name {
                            if filter.content.is_empty() {
                                log::info!("刪除篩選器 {}", name);
                                tag_filters.remove(i);
                            } else {
                                log::info!("修改篩選器 {:?}", filter);
                                f.obligation = *obligation;
                                f.filter = filter.content.clone();
                            }
                            found = true;
                            break;
                        }
                    }
                    if !found && !filter.content.is_empty() {
                        log::info!("新增篩選器 {:?}", filter);
                        conf.tag_filters.push(NamedTagFilter {
                            filter: filter.content.clone(),
                            obligation: *obligation,
                            name: name.clone(),
                        });
                    }
                } else {
                    log::info!("加入主篩選器 {:?}", filter);
                    conf.main_tag_filter = TagFilter {
                        filter: filter.content.clone(),
                        obligation: *obligation,
                    };
                }
            } else {
                println!("tag filters:");
                for filter in conf.tag_filters.iter() {
                    print!("  {} = [{}]", filter.name, filter.filter);
                    if filter.obligation {
                        print!(" (obligation)")
                    }
                    println!("")
                }
                println!("main tag filter:");
                print!("  [{}]", conf.main_tag_filter.filter);
                if conf.main_tag_filter.obligation {
                    print!(" (obligation)")
                }
                println!("")
            }
        }
        _ => unimplemented!(),
    }
    Ok(res)
}

async fn mv<'a, 'b>(
    mut entry: ScriptRepoEntry<'a, 'b>,
    new_name: Option<ScriptName<'a>>,
    ty: Option<&ScriptType>,
    tags: &Option<TagControlFlow>,
) -> Result {
    let og_script = path::open_script(&entry.name, &entry.ty, true)?;
    let new_script = path::open_script(
        new_name.as_ref().unwrap_or(&entry.name),
        ty.unwrap_or(&entry.ty),
        false,
    )?;
    util::mv(&og_script, &new_script)?;

    entry
        .update(|info| {
            if let Some(ty) = ty {
                info.ty = ty.clone();
            }
            if let Some(name) = new_name {
                info.name = name;
            }
            if let Some(tags) = tags {
                if tags.append {
                    log::debug!("附加上標籤：{:?}", tags);
                    info.tags.extend(tags.clone().into_allowed_iter());
                } else {
                    log::debug!("設定標籤：{:?}", tags);
                    info.tags = tags.clone().into_allowed_iter().collect();
                }
            }
            info.write();
        })
        .await
}
async fn edit_or_create<'a, 'b>(
    edit_query: &EditQuery,
    script_repo: &'b mut ScriptRepo<'a>,
    ty: Option<ScriptType>,
    tags: TagControlFlow,
) -> Result<(PathBuf, ScriptRepoEntry<'a, 'b>)> {
    let final_ty: ScriptType;
    let mut new_namespaces: Vec<Tag> = vec![];
    let (script_name, script_path) = if let EditQuery::Query(query) = edit_query {
        if let Some(entry) = query::do_script_query(query, script_repo)? {
            if let Some(ty) = ty {
                log::warn!("已存在的腳本無需再指定類型");
                if ty != entry.ty {
                    return Err(Error::CategoryMismatch {
                        expect: ty.clone(),
                        actual: entry.ty.clone(),
                    });
                }
            }
            final_ty = entry.ty.clone();
            log::debug!("打開既有命名腳本：{:?}", entry.name);
            let p = path::open_script(&entry.name, &entry.ty, true)
                .context(format!("打開命名腳本失敗：{:?}", entry.name))?;
            (entry.name.clone(), p)
        } else {
            final_ty = ty.unwrap_or_default();
            if script_repo
                .get_hidden_mut(&query.as_script_name()?)
                .is_some()
            {
                log::error!("與被篩掉的腳本撞名");
                return Err(Error::ScriptExist(query.as_script_name()?.to_string()));
            }

            log::debug!("打開新命名腳本：{:?}", query);
            let name = query.as_script_name()?;
            new_namespaces = name
                .namespaces()
                .iter()
                .map(|s| Tag::from_str(s))
                .collect::<Result<Vec<_>>>()?;

            let p = path::open_script(&name, &final_ty, false)
                .context(format!("打開新命名腳本失敗：{:?}", query))?;
            (name, p)
        }
    } else {
        final_ty = ty.unwrap_or_default();
        log::debug!("打開新匿名腳本");
        path::open_new_anonymous(&final_ty).context("打開新匿名腳本失敗")?
    };
    log::info!("編輯 {:?}", script_name);

    let entry = script_repo
        .upsert(&script_name, || {
            ScriptInfo::builder(
                0,
                script_name.clone().into_static(),
                final_ty,
                tags.into_allowed_iter().chain(new_namespaces.into_iter()),
            )
            .build()
        })
        .await?;
    Ok((script_path, entry))
}
