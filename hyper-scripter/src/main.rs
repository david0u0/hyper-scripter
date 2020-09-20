use chrono::Utc;
use hyper_scripter::config::{Config, NamedTagFilter};
use hyper_scripter::error::{Contextable, Error, Result};
use hyper_scripter::historian::{self, Event, EventData};
use hyper_scripter::list::{fmt_list, DisplayScriptIdent, DisplayStyle, ListOptions, ListPattern};
use hyper_scripter::query::{EditQuery, FilterQuery, ScriptQuery};
use hyper_scripter::script::{AsScriptName, ScriptInfo, ScriptName};
use hyper_scripter::script_repo::{ScriptRepo, ScriptRepoEntry};
use hyper_scripter::script_type::ScriptType;
use hyper_scripter::tag::{TagControlFlow, TagFilter, TagFilterGroup};
use hyper_scripter::{fuzzy, path, util};
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
    #[structopt(short, long, parse(try_from_str))]
    tags: Option<TagControlFlow>,
    #[structopt(short, long, help = "Shorthand for `-t=all,^deleted`")]
    all: bool,
    #[structopt(subcommand)]
    subcmd: Option<Subs>,
}
#[derive(StructOpt, Debug)]
enum Subs {
    #[structopt(external_subcommand)]
    Other(Vec<String>),
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
            short,
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
    #[structopt(about = "Execute the script query and get the exact file", settings = NO_FLAG_SETTINGS)]
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
        script_queries: Vec<ScriptQuery>,
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
            help = "Category type of the script, e.g. `sh`"
        )]
        category: Option<ScriptType>,
        #[structopt(short, long)]
        tags: Option<TagControlFlow>,
        #[structopt(parse(try_from_str))]
        origin: ScriptQuery,
        new: Option<String>,
    },
    #[structopt(
        about = "Manage script tags. If a list of tag is given, set it as default, otherwise show tag information."
    )]
    Tags {
        #[structopt(long, short, requires("filter"))]
        obligation: bool,
        #[structopt(parse(try_from_str))]
        filter: Option<FilterQuery>,
    },
}

impl Root {
    fn all(&self) -> bool {
        if self.all {
            return true;
        }
        if let Some(Subs::LS(List { all, .. })) = self.subcmd {
            all
        } else {
            false
        }
    }
}

#[derive(StructOpt, Debug)]
struct List {
    // TODO: 滿滿的其它排序/篩選選項
    #[structopt(short, long, help = "Show all scripts.")]
    all: bool,
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
    pattern: Option<ListPattern>,
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
fn get_info_mut<'b, 'a>(
    script_query: &ScriptQuery,
    script_repo: &'b mut ScriptRepo<'a>,
) -> Result<Option<ScriptRepoEntry<'a, 'b>>> {
    log::debug!("開始尋找 `{:?}`", script_query);
    match script_query {
        ScriptQuery::Prev(prev) => {
            let latest = script_repo.latest_mut(*prev);
            log::trace!("找最新腳本");
            return if latest.is_some() {
                Ok(latest)
            } else {
                Err(Error::Empty)
            };
        }
        ScriptQuery::Exact(name) => Ok(script_repo.get_mut(name)),
        ScriptQuery::Fuzz(name) => fuzzy::fuzz_mut(name, script_repo.iter_mut()),
    }
}
fn get_info_mut_strict<'b, 'a>(
    script_query: &ScriptQuery,
    script_repo: &'b mut ScriptRepo<'a>,
) -> Result<ScriptRepoEntry<'a, 'b>> {
    match get_info_mut(script_query, script_repo) {
        Err(e) => Err(e),
        Ok(None) => Err(Error::ScriptNotFound(
            script_query.as_script_name()?.to_string(),
        )),
        Ok(Some(info)) => Ok(info),
    }
}
async fn main_inner(root: &Root, conf: &mut Config) -> Result<Vec<Error>> {
    let pool = hyper_scripter::db::get_pool().await?;
    let mut repo = ScriptRepo::new(pool.clone())
        .await
        .context("讀取歷史記錄失敗")?;
    let mut res = Vec::<Error>::new();
    {
        let tag_group: TagFilterGroup = if root.all() {
            TagFilter::from_str("all,^deleted").unwrap().into()
        } else {
            match root.tags.clone() {
                Some(filter) => Into::<TagFilter>::into(filter).into(),
                None => conf.get_tag_filter_group(),
            }
        };
        repo.filter_by_tag(&tag_group);
    }

    match root.subcmd.as_ref().unwrap() {
        Subs::Edit {
            edit_query,
            category: ty,
            fast,
            content,
            no_template,
        } => {
            let edit_tags = root
                .tags
                .clone()
                .unwrap_or(conf.main_tag_filter.clone().filter);
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
            let mut entry = get_info_mut_strict(script_query, &mut repo)?;
            log::info!("執行 {:?}", entry.name);
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
            historian::record(
                Event {
                    data: EventData::ExecDone(ret_code),
                    script_id: entry.id,
                },
                &pool,
            )
            .await?;
        }
        Subs::Which { script_query } => {
            let entry = get_info_mut_strict(script_query, &mut repo)?;
            log::info!("定位 {:?}", entry.name);
            println!("{}", entry.file_path()?.to_string_lossy());
        }
        Subs::Cat { script_query } => {
            let mut entry = get_info_mut_strict(script_query, &mut repo)?;
            let script_path = path::open_script(&entry.name, &entry.ty, true)?;
            log::info!("打印 {:?}", entry.name);
            let content = util::read_file(&script_path)?;
            println!("{}", content);
            entry.update(|info| info.read()).await?;
        }
        Subs::LS(List {
            long,
            no_grouping,
            pattern,
            plain,
            name,
            file,
            all: _,
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
                pattern,
                display_style,
            };
            let stdout = std::io::stdout();
            fmt_list(&mut stdout.lock(), &mut repo, &opt)?;
        }
        Subs::RM { script_queries } => {
            let delete_tag: Option<TagControlFlow> = Some(FromStr::from_str("deleted").unwrap());
            for query in script_queries.into_iter() {
                let entry = get_info_mut_strict(query, &mut repo)?;
                // TODO: 若是模糊搜出來的，問一下使用者是不是真的要刪
                let script_path = path::open_script(&entry.name, &entry.ty, true)?;
                log::info!("刪除 {:?}", *entry);
                if entry.name.is_anonymous() {
                    log::debug!("刪除匿名腳本");
                    util::remove(&script_path)?;
                    let name = entry.name.clone().into_static();
                    repo.remove(&name).await?;
                } else {
                    log::debug!("不要真的刪除有名字的腳本，改用標籤隱藏之");
                    let time_str = Utc::now().format("%Y%m%d%H%M%S");
                    let new_name = util::change_name_only(&entry.name.to_string(), |name| {
                        format!("{}-{}", time_str, name)
                    });
                    mv(
                        query,
                        Some(new_name.as_script_name()?.into_static()), // TODO: 正確地實作 scriptname from string
                        &mut repo,
                        None,
                        &delete_tag,
                    )
                    .await?;
                }
            }
        }
        Subs::CP { origin, new } => {
            let h = get_info_mut_strict(origin, &mut repo)?;
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
            mv(origin, new_name, &mut repo, ty.as_ref(), tags).await?;
        }
        Subs::Tags { filter, obligation } => {
            if let Some(filter) = filter {
                if let Some(name) = &filter.name {
                    log::info!("加入篩選器 {:?}", filter);
                    let mut found = false;
                    for f in conf.tag_filters.iter_mut() {
                        if &f.name == name {
                            found = true;
                            f.obligation = *obligation;
                            f.filter = filter.content.clone();
                        }
                    }
                    if !found {
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
    origin: &ScriptQuery,
    new_name: Option<ScriptName<'a>>,
    script_repo: &'b mut ScriptRepo<'a>,
    ty: Option<&ScriptType>,
    tags: &Option<TagControlFlow>,
) -> Result {
    // FIXME: 避免 rm 時做兩次模糊搜尋
    let mut entry = get_info_mut_strict(origin, script_repo)?;
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
                info.tags = tags.clone().into_allowed_iter().collect();
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
    let (script_name, script_path) = if let EditQuery::Query(query) = edit_query {
        if let Some(entry) = get_info_mut(query, script_repo)? {
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
            ScriptInfo::new(
                0,
                script_name.clone().into_static(),
                final_ty,
                tags.into_allowed_iter(),
                None,
                None,
                None,
                None,
            )
        })
        .await?;
    Ok((script_path, entry))
}
