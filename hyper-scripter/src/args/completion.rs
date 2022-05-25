use super::NO_FLAG_SETTINGS;
use clap::Parser;

#[derive(Parser, Debug)]
pub enum Completion {
    #[clap(settings = NO_FLAG_SETTINGS)]
    LS {
        #[clap(long)]
        name: Option<String>, // NOTE: 不用 ScriptName，因為有 `hs/` 這種輸入要考慮
        #[clap(required = true, min_values = 1)]
        args: Vec<String>,
    },
    #[clap(settings = NO_FLAG_SETTINGS)]
    Alias {
        #[clap(required = true, min_values = 1)]
        args: Vec<String>,
    },
    #[clap(settings = NO_FLAG_SETTINGS)]
    Home {
        #[clap(required = true, min_values = 1)]
        args: Vec<String>,
    },
    #[clap(settings = NO_FLAG_SETTINGS)]
    ParseRun {
        #[clap(required = true, min_values = 1)]
        args: Vec<String>,
    },
    #[clap(settings = NO_FLAG_SETTINGS)]
    NoSubcommand {
        #[clap(required = true, min_values = 1)]
        args: Vec<String>,
    },
}

impl Completion {
    pub fn from_args(args: &[String]) -> Option<Completion> {
        let args = &args[1..];
        if args.first().map(AsRef::as_ref) == Some("completion") {
            log::info!("補全模式 {:?}", args);
            Some(Completion::from_iter(args))
        } else {
            None
        }
    }
}
