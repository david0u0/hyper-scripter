use clap::Parser;

#[derive(Parser, Debug)]
#[clap(
    disable_help_flag = true,
    allow_hyphen_values = true,
    trailing_var_arg = true
)]
pub enum Completion {
    LS {
        #[clap(long)]
        name: Option<String>, // NOTE: 不用 ScriptName，因為有 `hs/` 這種輸入要考慮
        #[clap(required = true, min_values = 1)]
        args: Vec<String>,
    },
    Alias {
        #[clap(required = true, min_values = 1)]
        args: Vec<String>,
    },
    Home {
        #[clap(required = true, min_values = 1)]
        args: Vec<String>,
    },
    ParseRun {
        #[clap(required = true, min_values = 1)]
        args: Vec<String>,
    },
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
