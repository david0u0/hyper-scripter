use chrono::Utc;
use instant_scripter::arg::ScriptArg;
use instant_scripter::config::Config;
use instant_scripter::error::{Contextabl, Error, Result};
use instant_scripter::history::History;
use instant_scripter::list::{fmt_list, ListOptions, ListPattern};
use instant_scripter::script::{AsScriptName, ScriptInfo, ScriptType};
use instant_scripter::tag::TagFilters;
use instant_scripter::{fuzzy, path, util};
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
    #[structopt(short = "p", long, help = "Path to instant script root")]
    is_path: Option<String>,
    #[structopt(short, long, parse(try_from_str))]
    tags: Option<TagFilters>,
    #[structopt(short, long, help = "Shorthand for `-t=all`")]
    all: bool,
    #[structopt(subcommand)]
    subcmd: Option<Subs>,
}
#[derive(StructOpt, Debug)]
enum Subs {
    #[structopt(external_subcommand)]
    Other(Vec<String>),
    #[structopt(about = "Edit instant script", alias = "e")]
    Edit {
        #[structopt(short, long, help = "The content for your script.")]
        content: Option<String>,
        #[structopt(parse(try_from_str))]
        script_name: Option<ScriptArg>,
        #[structopt(
            long,
            short = "x",
            parse(try_from_str),
            help = "Executable type of the script, e.g. `sh`"
        )]
        executable: Option<ScriptType>,
    },
    #[structopt(about = "Run the script", settings = NO_FLAG_SETTINGS)]
    Run {
        #[structopt(default_value = "-", parse(try_from_str))]
        script_name: ScriptArg,
        #[structopt(help = "Command line args to pass to the script.")]
        args: Vec<String>,
    },
    #[structopt(about = "Print the script to standard output.")]
    Cat {
        #[structopt(default_value = "-", parse(try_from_str))]
        script_name: ScriptArg,
    },
    #[structopt(about = "Remove the script")]
    RM {
        #[structopt(parse(try_from_str), required = true, min_values = 1)]
        scripts: Vec<ScriptArg>,
    },
    #[structopt(about = "List instant scripts", alias = "l")]
    LS(List),
    #[structopt(about = "Copy the script to another one")]
    CP {
        #[structopt(parse(try_from_str))]
        origin: ScriptArg,
        new: String,
    },
    #[structopt(about = "Move the script to another one")]
    MV {
        #[structopt(
            long,
            short = "x",
            parse(try_from_str),
            help = "Executable type of the script, e.g. `sh`"
        )]
        executable: Option<ScriptType>,
        #[structopt(short, long)]
        tags: Option<TagFilters>,
        #[structopt(parse(try_from_str))]
        origin: ScriptArg,
        new: Option<String>,
    },
    #[structopt(
        about = "Manage script tags. If a list of tag is given, set it as default, otherwise show tag information."
    )]
    Tags { tags: Option<TagFilters> },
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
    #[structopt(parse(try_from_str))]
    pattern: Option<ListPattern>,
}

fn main() -> std::result::Result<(), Vec<Error>> {
    env_logger::init();
    match main_err_handle() {
        Err(e) => Err(vec![e]),
        Ok(v) => {
            if v.len() == 0 {
                Ok(())
            } else {
                Err(v)
            }
        }
    }
}
fn main_err_handle() -> Result<Vec<Error>> {
    let mut root = Root::from_args();
    log::debug!("命令行物件：{:?}", root);
    match &root.is_path {
        Some(is_path) => path::set_path(is_path)?,
        None => path::set_path_from_sys()?,
    }
    let mut conf = Config::load()?;

    let mut hs = path::get_history().context("讀取歷史記錄失敗")?;
    if root.tags.is_none() {
        root.tags = Some(conf.tag_filters.clone());
    }
    if !root.all() {
        hs.filter_by_group(root.tags.as_ref().unwrap());
    }

    match root.subcmd {
        None => {
            root.subcmd = Some(Subs::Edit {
                script_name: Some(ScriptArg::Prev(1)),
                executable: None,
                content: None,
            });
        }
        Some(Subs::Other(args)) => {
            log::info!("執行模式");
            let run = Subs::Run {
                script_name: std::str::FromStr::from_str(&args[0])?,
                args: args[1..args.len()].iter().map(|s| s.clone()).collect(),
            };
            root.subcmd = Some(run);
        }
        _ => (),
    }
    let res = main_inner(&root, &mut hs, &mut conf)?;
    conf.store()?;
    path::store_history(hs.into_iter_all())?;
    Ok(res)
}
fn get_info_mut<'b, 'a>(
    script_name: &ScriptArg,
    history: &'b mut History<'a>,
) -> Result<Option<&'b mut ScriptInfo<'a>>> {
    log::debug!("開始尋找 `{:?}`", script_name);
    match script_name {
        ScriptArg::Prev(prev) => {
            let latest = history.latest_mut(*prev);
            log::trace!("找最新腳本");
            return if latest.is_some() {
                Ok(latest)
            } else {
                Err(Error::Empty)
            };
        }
        ScriptArg::Exact(name) => Ok(history.get_mut(name)),
        ScriptArg::Fuzz(name) => fuzzy::fuzz_mut(name, history.iter_mut()),
    }
}
fn get_info_mut_strict<'b, 'a>(
    script_name: &ScriptArg,
    history: &'b mut History<'a>,
) -> Result<&'b mut ScriptInfo<'a>> {
    match get_info_mut(script_name, history) {
        Err(e) => Err(e),
        Ok(None) => Err(Error::NoInfo(script_name.as_script_name()?.to_string())),
        Ok(Some(info)) => Ok(info),
    }
}
fn main_inner<'a>(root: &Root, hs: &mut History<'a>, conf: &mut Config) -> Result<Vec<Error>> {
    let mut res = Vec::<Error>::new();
    let tags = root.tags.clone().unwrap();

    match root.subcmd.as_ref().unwrap() {
        Subs::Edit {
            script_name,
            executable: ty,
            content,
        } => {
            let final_ty: ScriptType;
            let script = if let Some(name) = script_name {
                if let Some(h) = get_info_mut(name, hs)? {
                    if let Some(ty) = ty {
                        log::warn!("已存在的腳本無需再指定類型");
                        if ty != &h.ty {
                            return Err(Error::TypeMismatch {
                                expect: *ty,
                                actual: h.ty,
                            });
                        }
                    }
                    final_ty = h.ty;
                    log::debug!("打開既有命名腳本：{:?}", name);
                    path::open_script(&h.name, h.ty, true)
                        .context(format!("打開命名腳本失敗：{:?}", name))?
                } else {
                    final_ty = ty.unwrap_or_default();
                    log::debug!("打開新命名腳本：{:?}", name);
                    path::open_script(name, ty.unwrap_or_default(), false)
                        .context(format!("打開新命名腳本失敗：{:?}", name))?
                }
            } else {
                final_ty = ty.unwrap_or_default();
                log::debug!("打開新匿名腳本");
                path::open_new_anonymous(ty.unwrap_or_default()).context("打開新匿名腳本失敗")?
            };

            if let Some(content) = content {
                log::info!("快速編輯 {:?}", script.name);
                if script.path.exists() {
                    return Err(Error::PathExist(script.path));
                }
                util::fast_write_script(&script, content)?;
            } else {
                log::info!("編輯 {:?}", script.name);
                util::prepare_script(&script.path, final_ty)?;
                let cmd = util::create_cmd("vim", &[script.path]);
                let stat = util::run_cmd("vim", cmd)?;
                log::debug!("編輯器返回：{:?}", stat);
            }

            let name = script.name.into_static();
            let h = hs.entry(&name).or_insert(ScriptInfo::new(
                name,
                final_ty,
                tags.into_allowed_iter(),
            )?);
            h.read();
        }
        Subs::Run { script_name, args } => {
            let h = get_info_mut_strict(script_name, hs)?;
            log::info!("執行 {:?}", h.name);
            let script = path::open_script(&h.name, h.ty, true)?;
            match util::run(&script, &h, &args) {
                Err(e @ Error::ScriptError(_)) => res.push(e),
                Err(e) => return Err(e),
                Ok(_) => (),
            }
            h.exec();
        }
        Subs::Cat { script_name } => {
            let h = get_info_mut_strict(script_name, hs)?;
            let script = path::open_script(&h.name, h.ty, true)?;
            log::info!("打印 {:?}", script.name);
            let content = util::read_file(&script.path)?;
            println!("{}", content);
            h.read();
        }
        Subs::LS(list) => {
            let opt = ListOptions {
                long: list.long,
                pattern: &list.pattern,
            };
            let stdout = std::io::stdout();
            fmt_list(&mut stdout.lock(), hs, &opt)?;
        }
        Subs::RM { scripts } => {
            for script_name in scripts.into_iter() {
                let h = get_info_mut_strict(script_name, hs)?;
                // TODO: 若是模糊搜出來的，問一下使用者是不是真的要刪
                let script = path::open_script(&h.name, h.ty, true)?;
                log::info!("刪除 {:?}", script);
                util::remove(&script)?;
                let name = script.name.into_static();
                hs.remove(&name);
            }
        }
        Subs::CP { origin, new } => {
            let h = get_info_mut_strict(origin, hs)?;
            let new_name = new.as_script_name()?;
            let og_script = path::open_script(&h.name, h.ty, true)?;
            let new_script = path::open_script(&new_name, h.ty, false)?;
            if new_script.path.exists() {
                return Err(Error::PathExist(new_script.path));
            }
            util::cp(&og_script, &new_script)?;
            let new_info = ScriptInfo {
                name: new_name.into_static(),
                read_time: Utc::now(),
                ..h.clone()
            };
            hs.insert(new_info);
        }
        Subs::MV {
            origin,
            new,
            tags,
            executable: ty,
        } => {
            let h = get_info_mut_strict(origin, hs)?;
            let og_script = path::open_script(&h.name, h.ty, true)?;
            let new_ty = ty.unwrap_or(h.ty);
            let new_name = match new {
                Some(s) => s.as_script_name()?,
                None => h.name.clone(),
            };
            let new_script = path::open_script(&new_name, new_ty, false)?;
            util::mv(&og_script, &new_script)?;

            h.name = new_name.into_static();
            h.read();
            h.ty = new_ty;
            if let Some(tags) = tags {
                h.tags = tags.clone().into_allowed_iter().collect();
            }
        }
        Subs::Tags { tags } => {
            if let Some(tags) = tags {
                conf.tag_filters = tags.clone();
            } else {
                println!("current tag filter = [{}]", conf.tag_filters);
            }
        }
        _ => unimplemented!(),
    }
    Ok(res)
}
