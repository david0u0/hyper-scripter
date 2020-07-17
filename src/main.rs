mod error;
mod path;
mod script;

use error::Result;
use std::process::Command;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Root {
    #[structopt(short = "p", long, help = "path to flash script root")]
    fs_path: Option<String>,
    #[structopt(subcommand)]
    subcmd: Option<Subs>,
}
#[derive(StructOpt, Debug)]
enum Subs {
    #[structopt(about = "Edit flash script", alias = "e")]
    Edit { script_name: Option<String> },
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
        Subs::Edit { script_name: None }
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
    match sub {
        Subs::Edit { script_name } => {
            let script = if let Some(name) = script_name {
                path::open_script(name, false)?
            } else {
                path::open_anonymous_script(None, false)?
            };
            let mut cmd = Command::new("vim");
            cmd.args(&[script.path]).spawn()?.wait()?;
        }
        Subs::Run { script_name, args } => {
            let script = path::open_script(script_name, true)?;
            let mut cmd = Command::new("sh");
            let mut full_args = vec![script.path];
            full_args.extend(args.into_iter().map(|s| s.into()));
            cmd.args(full_args).spawn()?.wait()?;
        }
        _ => unimplemented!(),
    }
    Ok(())
}
