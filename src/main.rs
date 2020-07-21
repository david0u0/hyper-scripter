mod error;
mod fuzzy;
mod history;
mod list;
mod path;
mod script;
mod tag;
mod util;

use chrono::Utc;
use error::{Contextabl, Error, Result};
use history::History;
use list::{fmt_list, ListOptions, ListPattern};
use script::{ScriptInfo, ScriptType, ToScriptName};
use std::process::Command;
use structopt::clap::AppSettings::{
    self, AllowLeadingHyphen, DisableHelpFlags, DisableHelpSubcommand, DisableVersion,
    TrailingVarArg,
};
use structopt::StructOpt;
use tag::TagFilters;

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
    #[structopt(short, long, parse(try_from_str), default_value = "g,-h")]
    tags: TagFilters,
    #[structopt(subcommand)]
    subcmd: Option<Subs>,
}
#[derive(StructOpt, Debug)]
enum Subs {
    #[structopt(external_subcommand)]
    Other(Vec<String>),
    #[structopt(about = "Edit instant script", alias = "e")]
    Edit {
        #[structopt(short, long, help = "Don't do fuzzy search")]
        exact: bool,
        #[structopt(short, long, help = "Hide the script in list")]
        hide: bool,
        script_name: Option<String>,
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
        #[structopt(default_value = "-")]
        script_name: String,
        #[structopt(help = "Command line args to pass to the script.")]
        args: Vec<String>,
    },
    #[structopt(about = "Print the script to standard output.")]
    Cat {
        #[structopt(default_value = "-")]
        script_name: String,
    },
    #[structopt(about = "Remove the script")]
    RM {
        #[structopt(short, long, help = "Don't do fuzzy search")]
        exact: bool,
        #[structopt(required = true, min_values = 1)]
        scripts: Vec<String>,
    },
    #[structopt(about = "List instant scripts", alias = "l")]
    LS(List),
    #[structopt(about = "Move the script to another one", alias = "mv")]
    CP {
        #[structopt(short, long, help = "Don't do fuzzy search")]
        exact: bool,
        origin: String,
        new: String,
    },
    MV {
        #[structopt(short, long, help = "Don't do fuzzy search")]
        exact: bool,
        #[structopt(
            long,
            short = "x",
            parse(try_from_str),
            help = "Executable type of the script, e.g. `sh`"
        )]
        executable: Option<ScriptType>,
        origin: String,
        new: String,
    },
}

#[derive(StructOpt, Debug)]
struct List {
    // TODO: 滿滿的其它排序/篩選選項
    #[structopt(short, long, help = "Show all files including hidden ones.")]
    all: bool,
    #[structopt(short, long, help = "Show verbose information.")]
    long: bool,
    #[structopt(parse(try_from_str))]
    pattern: Option<ListPattern>,
}

fn main() -> Result<()> {
    env_logger::init();
    let mut root = Root::from_args();
    main_inner(&mut root)
}
fn get_info_mut<'a>(
    name: &str,
    history: &'a mut History,
    exact: bool,
) -> Result<Option<&'a mut ScriptInfo>> {
    log::trace!("開始尋找 `{}`, exact={}", name, exact);
    if name == "-" {
        log::trace!("找最新腳本");
        let latest = history.latest_mut();
        if latest.is_some() {
            Ok(latest)
        } else {
            Err(Error::Empty)
        }
    } else if exact {
        let name = name
            .clone() // TODO: Cow 優化
            .to_script_name()?;
        Ok(history.get_mut(&name))
    } else {
        fuzzy::fuzz(name, history.iter_mut())
    }
}
fn get_info_mut_strict<'a>(
    name: &str,
    history: &'a mut History,
    exact: bool,
) -> Result<&'a mut ScriptInfo> {
    match get_info_mut(name, history, exact) {
        Err(e) => Err(e),
        Ok(None) => Err(Error::NoInfo(name.to_owned())),
        Ok(Some(info)) => Ok(info),
    }
}
fn main_inner(root: &mut Root) -> Result<()> {
    log::debug!("命令行物件：{:?}", root);
    match &root.is_path {
        Some(is_path) => path::set_path(is_path)?,
        None => path::set_path(path::join_path(".", &path::get_sys_path()?)?)?,
    }

    let edit_last = Subs::Edit {
        script_name: Some("-".to_owned()),
        exact: false,
        hide: false,
        executable: None,
    };
    let sub = root.subcmd.as_ref().unwrap_or(&edit_last);

    let mut hs = path::get_history().context("讀取歷史記錄失敗")?;
    match sub {
        Subs::Other(cmds) => {
            log::info!("純執行模式");
            let run = Subs::Run {
                script_name: cmds[0].clone(),
                args: cmds[1..cmds.len()].iter().map(|s| s.clone()).collect(),
            };
            root.subcmd = Some(run);
            return main_inner(root);
        }
        Subs::Edit {
            script_name,
            hide,
            executable: ty,
            exact,
        } => {
            let script = if let Some(name) = script_name {
                if let Some(h) = get_info_mut(name, &mut hs, *exact)? {
                    if let Some(ty) = ty {
                        log::warn!("已存在的腳本無需再指定類型");
                        if ty != &h.ty {
                            return Err(Error::TypeMismatch {
                                expect: *ty,
                                actual: h.ty,
                            });
                        }
                    }
                    log::debug!("打開既有命名腳本：{:?}", name);
                    path::open_script(h.name.clone(), h.ty, true)
                        .context(format!("打開命名腳本失敗：{:?}", name))?
                } else {
                    log::debug!("打開新命名腳本：{:?}", name);
                    path::open_script(name.clone(), ty.unwrap_or_default(), false)
                        .context(format!("打開新命名腳本失敗：{:?}", name))?
                }
            } else {
                log::debug!("打開新匿名腳本");
                path::open_new_anonymous(ty.unwrap_or_default()).context("打開新匿名腳本失敗")?
            };

            log::info!("編輯 {:?}", script.name);
            let mut cmd = Command::new("vim");
            cmd.args(&[script.path]).spawn()?.wait()?;

            let dir = util::handle_fs_err(&["."], std::env::current_dir())?;
            let h = hs.entry(script.name.clone()).or_insert(ScriptInfo::new(
                script.name,
                ty.unwrap_or_default(),
                dir,
            )?);
            h.hidden = *hide;
            h.edit_time = Utc::now();
        }
        Subs::Run { script_name, args } => {
            let h = get_info_mut_strict(script_name, &mut hs, false)?;
            log::info!("執行 {:?}", h.name);
            let script = path::open_script(&h.name, h.ty, true)?;
            util::run(&script, &h, &args)?;
            h.exec_time = Some(Utc::now());
        }
        Subs::Cat { script_name } => {
            let h = get_info_mut_strict(script_name, &mut hs, false)?;
            let script = path::open_script(&h.name, h.ty, true)?;
            log::info!("打印 {:?}", script.name);
            let content = util::read_file(&script.path)?;
            println!("{}", content);
        }
        Subs::LS(list) => {
            let opt = ListOptions {
                long: list.long,
                pattern: &list.pattern,
            };
            let stdout = std::io::stdout();
            fmt_list(&mut stdout.lock(), hs.into_iter(), &opt)?;
            return Ok(());
        }
        Subs::RM { scripts, exact } => {
            for script_name in scripts.into_iter() {
                let h = get_info_mut_strict(script_name, &mut hs, *exact)?;
                // TODO: 若是模糊搜出來的，問一下使用者是不是真的要刪
                let script = path::open_script(&h.name, h.ty, true)?;
                util::remove(&script)?;
                hs.remove(&script.name);
            }
        }
        Subs::CP { exact, origin, new } => {
            let h = get_info_mut_strict(origin, &mut hs, *exact)?;
            let new_name = new.clone().to_script_name()?;
            let og_script = path::open_script(&h.name, h.ty, true)?;
            let new_script = path::open_script(&new_name, h.ty, false)?;
            if new_script.path.exists() {
                return Err(Error::PathExist(new_script.path));
            }
            util::cp(&og_script, &new_script)?;
            let new_info = ScriptInfo {
                name: new_name,
                birthplace: util::handle_fs_err(&["."], std::env::current_dir())?,
                edit_time: Utc::now(),
                ..h.clone()
            };
            hs.insert(new_info);
        }
        Subs::MV {
            exact,
            origin,
            new,
            executable: ty,
        } => {
            let h = get_info_mut_strict(origin, &mut hs, *exact)?;
            let og_script = path::open_script(&h.name, h.ty, true)?;
            let new_ty = ty.unwrap_or(h.ty);
            let new_name = new.clone().to_script_name()?;
            let new_script = path::open_script(&new_name, new_ty, false)?;
            util::mv(&og_script, &new_script)?;

            h.edit_time = Utc::now();
            h.name = new_name;
            h.ty = new_ty;
            h.birthplace = util::handle_fs_err(&["."], std::env::current_dir())?;
        }
    }
    path::store_history(hs.into_iter())?;
    Ok(())
}
