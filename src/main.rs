mod error;
mod fuzzy;
mod list;
mod path;
mod script;
mod util;

use chrono::Utc;
use error::{Contextabl, Error, Result};
use list::{fmt_list, ListOptions};
use script::{ScriptMeta, ScriptType, ToScriptName};
use std::process::Command;
use structopt::StructOpt;
use util::{map_to_iter, run};

#[derive(StructOpt, Debug)]
struct Root {
    #[structopt(short = "p", long, help = "Path to instant script root")]
    is_path: Option<String>,
    #[structopt(subcommand)]
    subcmd: Option<Subs>,
}
#[derive(StructOpt, Debug)]
enum Subs {
    #[structopt(about = "Edit instant script", alias = "e")]
    Edit {
        #[structopt(short, long, help = "Hide the script in list")]
        hide: bool,
        script_name: Option<String>,
        #[structopt(short, parse(try_from_str), help = "Type of the script, e.g. `sh`")]
        ty: Option<ScriptType>,
    },
    #[structopt(about = "Edit the last script. This is the default subcommand.")]
    EditLast,
    #[structopt(about = "Run the last script edited or run", alias = ".")]
    RunLast {
        #[structopt(help = "Command line args to pass to the script.")]
        args: Vec<String>,
    },
    #[structopt(about = "Run the script", alias = "r")]
    Run {
        #[structopt(
            help = "The script's name. Prefix `.` to specify anonymous scripts, such as `run .42`"
        )]
        script_name: String,
        #[structopt(help = "Command line args to pass to the script.")]
        args: Vec<String>,
    },
    #[structopt(about = "List instant scripts", aliases = &["l", "ls"])]
    List(List),
    #[structopt(about = "Move the script to another one", alias = "mv")]
    Move { origin: String, target: String },
}

#[derive(StructOpt, Debug)]
struct List {
    // TODO: 滿滿的其它排序/篩選選項
    #[structopt(short, long, help = "Show all files including hidden ones.")]
    all: bool,
}

#[derive(StructOpt, Debug)]
struct RootOnlyRun {
    script_name: String,
    #[structopt(help = "Command line args to pass to the script.")]
    args: Vec<String>,
    #[structopt(short = "p", long, help = "Path to instant script root")]
    is_path: Option<String>,
}

fn main() -> Result<()> {
    env_logger::init();
    match Root::from_iter_safe(std::env::args()) {
        Ok(root) => main_inner(root),
        Err(_) => {
            log::info!("純執行模式");
            main_only_run()
        }
    }
}
fn main_only_run() -> Result<()> {
    let only_run = RootOnlyRun::from_iter(std::env::args());
    let root = Root {
        is_path: only_run.is_path,
        subcmd: Some(Subs::Run {
            args: only_run.args,
            script_name: only_run.script_name,
        }),
    };
    main_inner(root)
}
fn main_inner(root: Root) -> Result<()> {
    if let Some(is_path) = root.is_path {
        path::set_path(is_path)?;
    } else {
        path::set_path(path::join_path(".", &path::get_sys_path()?)?)?;
    }

    let sub = root.subcmd.unwrap_or(Subs::EditLast);
    let mut hs = path::get_history().context("讀取歷史記錄失敗")?;
    let latest = hs.iter_mut().max_by_key(|(_, h)| h.last_time());
    match sub {
        Subs::Edit {
            script_name,
            hide,
            ty,
        } => {
            let mut actual_ty = ty.unwrap_or_default();
            let script = if let Some(name) = script_name {
                let name = name.to_script_name()?;
                if let Some(h) = hs.get(&name) {
                    actual_ty = h.ty;
                    if let Some(ty) = ty {
                        log::warn!("已存在的腳本無需再指定類型");
                        if ty != h.ty {
                            return Err(Error::TypeMismatch {
                                expect: ty,
                                actual: h.ty,
                            });
                        }
                    }
                }
                path::open_script(name.clone(), actual_ty, false)
                    .context(format!("打開指定腳本失敗：{:?}", name))?
            } else {
                path::open_anonymous_script(None, actual_ty, false).context("打開新匿名腳本失敗")?
            };
            let h = hs
                .entry(script.name.clone())
                .or_insert(ScriptMeta::new(script.name, actual_ty)?);
            // FIXME: 重覆的東西抽一抽啦
            let mut cmd = Command::new("vim");
            cmd.args(&[script.path]).spawn()?.wait()?;
            h.hidden = hide;
            h.edit_time = Utc::now();
            path::store_history(map_to_iter(hs))?;
        }
        Subs::EditLast => {
            log::info!("嘗試打開最新的腳本…");
            let script = if let Some((_, latest)) = latest {
                path::open_script(latest.name.clone(), latest.ty, false)
                    .context(format!("打開最新腳本失敗：{:?}", latest.name))?
            } else {
                log::info!("沒有最近歷史，改為創建新的匿名腳本");
                path::open_anonymous_script(None, Default::default(), false)
                    .context("打開新匿名腳本失敗")?
            };
            let mut cmd = Command::new("vim");
            cmd.args(&[script.path]).spawn()?.wait()?;
            let h = hs
                .entry(script.name.clone())
                .or_insert(ScriptMeta::new(script.name, Default::default())?);
            h.edit_time = Utc::now();
            path::store_history(map_to_iter(hs))?;
        }
        Subs::Run { script_name, args } => {
            let h = fuzzy::fuzz_mut(&script_name, &mut hs)?.ok_or(Error::NoMeta(script_name))?;
            log::info!("執行 {:?}", h.name);
            let script = path::open_script(&h.name, h.ty, true)?;
            run(&script, h.ty, &args)?;
            h.exec_time = Some(Utc::now());
            path::store_history(map_to_iter(hs))?;
        }
        Subs::RunLast { args } => {
            // FIXME: Script 跟 ScriptType 分兩個地方太瞎了，早晚要合回去
            if let Some((_, latest)) = latest {
                let script = path::open_script(latest.name.clone(), latest.ty, false)
                    .context(format!("打開最新腳本失敗：{:?}", latest.name))?;
                run(&script, latest.ty, &args)?;
                latest.exec_time = Some(Utc::now());
                path::store_history(map_to_iter(hs))?;
            } else {
                return Err(Error::Empty);
            };
        }
        Subs::List(list) => {
            let opt = ListOptions {
                show_hidden: list.all,
            };
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            fmt_list(&mut handle, map_to_iter(hs), &opt)?;
        }
        _ => unimplemented!(),
    }
    Ok(())
}
