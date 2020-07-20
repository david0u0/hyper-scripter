mod error;
mod fuzzy;
mod list;
mod path;
mod script;
mod util;

use chrono::Utc;
use error::{Contextabl, Error, Result};
use list::{fmt_list, ListOptions};
use script::{CommandType, ScriptInfo};
use std::process::Command;
use structopt::clap::AppSettings::{
    self, AllowLeadingHyphen, DisableHelpFlags, DisableHelpSubcommand, DisableVersion,
    TrailingVarArg,
};
use structopt::StructOpt;
use util::{map_to_iter, run};

const NO_FLAG_SETTINGS: &[AppSettings] = &[
    AllowLeadingHyphen,
    DisableHelpFlags,
    TrailingVarArg,
    DisableHelpSubcommand,
    DisableVersion,
];

#[derive(StructOpt, Debug)]
struct Root {
    #[structopt(short = "p", long, help = "Path to instant script root")]
    is_path: Option<String>,
    #[structopt(short, long)]
    tags: Option<String>,
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
        #[structopt(short, parse(try_from_str), help = "Type of the script, e.g. `sh`")]
        command_type: Option<CommandType>,
    },
    #[structopt(about = "Edit the last script. This is the default subcommand.")]
    EditLast,
    #[structopt( alias = ".", about = "Run the last script edited or run", settings = NO_FLAG_SETTINGS)]
    RunLast {
        #[structopt(help = "Command line args to pass to the script.")]
        args: Vec<String>,
    },
    #[structopt(about = "Run the script", settings = NO_FLAG_SETTINGS)]
    Run {
        script_name: String,
        #[structopt(help = "Command line args to pass to the script.")]
        args: Vec<String>,
    },
    #[structopt(about = "Print the script to standard output.")]
    Cat { script_name: Option<String> },
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
    MV { origin: String, target: String },
}

#[derive(StructOpt, Debug)]
struct List {
    // TODO: 滿滿的其它排序/篩選選項
    #[structopt(short, long, help = "Show all files including hidden ones.")]
    all: bool,
    #[structopt(short, long, help = "Show verbose information.")]
    long: bool,
}

fn main() -> Result<()> {
    env_logger::init();
    let mut root = Root::from_args();
    main_inner(&mut root)
}
fn main_inner(root: &mut Root) -> Result<()> {
    log::debug!("命令行物件：{:?}", root);
    if let Some(is_path) = &root.is_path {
        path::set_path(is_path)?;
    } else {
        path::set_path(path::join_path(".", &path::get_sys_path()?)?)?;
    }

    let edit_last = Subs::EditLast;
    let sub = root.subcmd.as_ref().unwrap_or(&edit_last);

    let mut hs = path::get_history().context("讀取歷史記錄失敗")?;
    let latest = hs
        .iter_mut()
        .max_by_key(|(_, h)| h.last_time())
        .map(|h| h.1);
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
            command_type: ty,
            mut exact,
        } => {
            if ty.is_some() {
                exact = true;
            }
            let script = if let Some(name) = script_name {
                if let Some(h) = fuzzy::fuzz_mut(&name, &mut hs, exact)? {
                    if let Some(ty) = ty {
                        log::warn!("已存在的腳本無需再指定類型");
                        if ty != &h.ty {
                            return Err(Error::TypeMismatch {
                                expect: *ty,
                                actual: h.ty,
                            });
                        }
                    }
                    log::debug!("打開既有指定腳本：{:?}", name);
                    path::open_script(h.name.clone(), h.ty, true)
                        .context(format!("打開指定腳本失敗：{:?}", name))?
                } else {
                    log::debug!("打開新指定腳本：{:?}", name);
                    path::open_script(name.clone(), ty.unwrap_or_default(), false)
                        .context(format!("打開新指定腳本失敗：{:?}", name))?
                }
            } else {
                log::debug!("打開新匿名腳本");
                path::open_new_anonymous(ty.unwrap_or_default()).context("打開新匿名腳本失敗")?
            };

            log::info!("編輯 {:?}", script.name);
            let mut cmd = Command::new("vim");
            cmd.args(&[script.path]).spawn()?.wait()?;

            let h = hs
                .entry(script.name.clone())
                .or_insert(ScriptInfo::new(script.name, ty.unwrap_or_default())?);
            // FIXME: 重覆的東西抽一抽啦
            h.hidden = *hide;
            h.edit_time = Utc::now();
        }
        Subs::EditLast => {
            log::info!("嘗試打開最新的腳本…");
            let script = if let Some(latest) = latest {
                path::open_script(latest.name.clone(), latest.ty, false)
                    .context(format!("打開最新腳本失敗：{:?}", latest.name))?
            } else {
                log::info!("沒有最近歷史，改為創建新的匿名腳本");
                path::open_new_anonymous(Default::default()).context("打開新匿名腳本失敗")?
            };
            let mut cmd = Command::new("vim");
            cmd.args(&[script.path]).spawn()?.wait()?;
            let h = hs
                .entry(script.name.clone())
                .or_insert(ScriptInfo::new(script.name, Default::default())?);
            h.edit_time = Utc::now();
        }
        Subs::Run { script_name, args } => {
            let h = fuzzy::fuzz_mut(&script_name, &mut hs, false)?
                .ok_or(Error::NoMeta(script_name.clone()))?;
            log::info!("執行 {:?}", h.name);
            let script = path::open_script(&h.name, h.ty, true)?;
            run(&script, h.ty, &args)?;
            h.exec_time = Some(Utc::now());
        }
        Subs::RunLast { args } => {
            // FIXME: ScriptMeta 跟 CommandType 分兩個地方太瞎了，早晚要合回去
            if let Some(latest) = latest {
                let script = path::open_script(latest.name.clone(), latest.ty, false)
                    .context(format!("打開最新腳本失敗：{:?}", latest.name))?;
                run(&script, latest.ty, &args)?;
                latest.exec_time = Some(Utc::now());
            } else {
                return Err(Error::Empty);
            };
        }
        Subs::Cat { script_name } => {
            let script = if let Some(name) = script_name {
                let h = fuzzy::fuzz_mut(&name, &mut hs, false)?
                    .ok_or(Error::NoMeta(name.to_owned()))?;
                path::open_script(&h.name, h.ty, true)?
            } else if let Some(latest) = latest {
                path::open_script(&latest.name, latest.ty, true)?
            } else {
                return Err(Error::Empty);
            };
            log::info!("打印 {:?}", script.name);
            let content = util::read_file(&script.path)?;
            println!("{}", content);
        }
        Subs::LS(list) => {
            let opt = ListOptions {
                show_hidden: list.all,
                long: list.long,
            };
            let stdout = std::io::stdout();
            fmt_list(&mut stdout.lock(), map_to_iter(hs), &opt)?;
            return Ok(());
        }
        Subs::RM { scripts, exact } => {
            for script_name in scripts.into_iter() {
                let h = fuzzy::fuzz_mut(&script_name, &mut hs, *exact)?
                    .ok_or(Error::NoMeta(script_name.clone()))?;
                // TODO: 若是模糊搜出來的，問一下使用者是不是真的要刪
                let script = path::open_script(&h.name, h.ty, true)?;
                util::remove(&script)?;
                hs.remove(&script.name);
            }
        }
        _ => unimplemented!(),
    }
    path::store_history(map_to_iter(hs))?;
    Ok(())
}
