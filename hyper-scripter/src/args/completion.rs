use clap::Parser;
use std::num::NonZeroUsize;

#[derive(Parser, Debug)]
#[command(disable_help_flag = true, allow_hyphen_values = true)]
pub enum Completion {
    LS {
        #[arg(long)]
        limit: Option<NonZeroUsize>,
        #[arg(long)]
        bang: bool,
        #[arg(long)]
        name: Option<String>, // NOTE: 不用 ScriptName，因為有 `hs/` 這種輸入要考慮
        #[arg(required = true, num_args = 1..)]
        args: Vec<String>,
    },
    Alias {
        #[arg(required = true, num_args = 1..)]
        args: Vec<String>,
    },
    Home {
        #[arg(required = true, num_args = 1..)]
        args: Vec<String>,
    },
    ParseRun {
        #[arg(required = true, num_args = 1..)]
        args: Vec<String>,
    },
    NoSubcommand {
        #[arg(required = true, num_args = 1..)]
        args: Vec<String>,
    },
}

impl Completion {
    pub fn from_args(args: &[String]) -> Option<Completion> {
        let args = &args[1..];
        if args.first().map(AsRef::as_ref) == Some("completion") {
            log::info!("補全模式 {:?}", args);
            Some(Completion::parse_from(args))
        } else {
            None
        }
    }
}
