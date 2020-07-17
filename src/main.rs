mod error;
mod history;
mod path;
mod script;
mod util;

use chrono::Utc;
use error::{Contextabl, Error, Result};
use history::ScriptHistory;
use std::process::Command;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Root {
    #[structopt(short = "p", long, help = "Path to flash script root")]
    fs_path: Option<String>,
    #[structopt(subcommand)]
    subcmd: Option<Subs>,
}
#[derive(StructOpt, Debug)]
enum Subs {
    #[structopt(about = "Edit flash script", alias = "e")]
    Edit {
        #[structopt(short, long, help = "Create and edit a new anonymous script")]
        new: bool,
        #[structopt(
            help = "The script's name. Prefix `.` to specify anonymous scripts, such as `run .42`"
        )]
        script_name: Option<String>,
    },
    #[structopt(about = "Run the last script edited or run", alias = ".")]
    RunLast { args: Vec<String> },
    #[structopt(about = "Run the script", alias = "r")]
    Run {
        #[structopt(
            help = "The script's name. Prefix `.` to specify anonymous scripts, such as `run .42`"
        )]
        script_name: String,
        args: Vec<String>,
    },
    #[structopt(about = "List flash scripts", alias = "l")]
    List(List),
    #[structopt(about = "Move the script to another one", alias = "mv")]
    Move { origin: String, target: String },
}

impl Default for Subs {
    fn default() -> Self {
        Subs::Edit {
            script_name: None,
            new: false,
        }
    }
}

#[derive(StructOpt, Debug)]
struct List {
    #[structopt(short, long, help = "list all scripts")]
    all: bool,
}

fn main() -> Result<()> {
    env_logger::init();
    let root = Root::from_args();
    if let Some(fs_path) = root.fs_path {
        path::set_path(fs_path)?;
    }

    let sub = root.subcmd.unwrap_or_default();
    let mut hs = path::get_history().context("讀取歷史記錄失敗")?;
    let latest = hs.iter().max_by_key(|(_, h)| h.last_time());
    match sub {
        Subs::Edit { new, script_name } => {
            let script = if new {
                if script_name.is_some() {
                    return Err(Error::Operation(
                        "The --new flag shouldn't be set when there is a script name.".to_owned(),
                    ));
                }
                path::open_anonymous_script(None, false).context("打開新匿名腳本失敗")?
            } else if let Some(name) = script_name {
                path::open_script(name.clone(), false)
                    .context(format!("打開指定腳本失敗：{}", name))?
            } else {
                if let Some((_, latest)) = latest {
                    path::open_script(latest.name.clone(), false)
                        .context(format!("打開最新腳本失敗：{:?}", latest.name))?
                } else {
                    log::info!("沒有最近歷史，改為創建新的匿名腳本");
                    path::open_anonymous_script(None, false).context("打開新匿名腳本失敗")?
                }
            };
            let mut cmd = Command::new("vim");
            cmd.args(&[script.path]).spawn()?.wait()?;
            let h = hs
                .entry(script.name.clone())
                .or_insert(ScriptHistory::new(script.name)?);
            h.edit_time = Utc::now();
            path::store_history(hs)?;
        }
        Subs::Run { script_name, args } => {
            let script = path::open_script(script_name, true)?;
            util::run(&script, args)?;
            let h = hs
                .get_mut(&script.name.clone())
                .ok_or(Error::Format(format!("Missing history: {:?}", script.name)))?;
            h.exec_time = Some(Utc::now());
            path::store_history(hs)?;
        }
        Subs::RunLast { args } => {
            let script = if let Some((_, latest)) = latest {
                path::open_script(latest.name.clone(), false)
                    .context(format!("打開最新腳本失敗：{:?}", latest.name))?
            } else {
                return Err(Error::Empty);
            };
            util::run(&script, args)?;
            let h = hs
                .get_mut(&script.name.clone())
                .ok_or(Error::Format(format!("Missing history: {:?}", script.name)))?;
            h.exec_time = Some(Utc::now());
            path::store_history(hs)?;
        }
        _ => unimplemented!(),
    }
    Ok(())
}
