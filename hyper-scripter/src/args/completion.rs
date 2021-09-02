use super::NO_FLAG_SETTINGS;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub enum Completion {
    #[structopt(settings = NO_FLAG_SETTINGS)]
    LS { args: Vec<String> },
    #[structopt(settings = NO_FLAG_SETTINGS)]
    Alias { args: Vec<String> },
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
