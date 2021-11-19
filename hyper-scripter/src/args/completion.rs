use super::NO_FLAG_SETTINGS;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Completion {
    #[structopt(settings = NO_FLAG_SETTINGS)]
    LS {
        #[structopt(long)]
        name: Option<String>, // NOTE: 不用 ScriptName，因為有 `hs/` 這種輸入要考慮
        #[structopt(required = true, min_values = 1)]
        args: Vec<String>,
    },
    #[structopt(settings = NO_FLAG_SETTINGS)]
    Alias {
        #[structopt(required = true, min_values = 1)]
        args: Vec<String>,
    },
    #[structopt(settings = NO_FLAG_SETTINGS)]
    Home {
        #[structopt(required = true, min_values = 1)]
        args: Vec<String>,
    },
    #[structopt(settings = NO_FLAG_SETTINGS)]
    ParseRun {
        #[structopt(required = true, min_values = 1)]
        args: Vec<String>,
    },
    #[structopt(settings = NO_FLAG_SETTINGS)]
    NoSubcommand {
        #[structopt(required = true, min_values = 1)]
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
