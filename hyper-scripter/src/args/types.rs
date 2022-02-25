use super::help_str::*;
use crate::script_type::ScriptFullType;
use serde::Serialize;
use structopt::clap::AppSettings::AllowLeadingHyphen;
use structopt::StructOpt;

#[derive(StructOpt, Debug, Serialize)]
pub struct Types {
    #[structopt(subcommand)]
    pub subcmd: Option<TypesSubs>,
}

#[derive(StructOpt, Debug, Serialize)]
#[structopt(settings = &[AllowLeadingHyphen])]
pub enum TypesSubs {
    #[structopt(external_subcommand)]
    Other(Vec<String>),
    LS {
        #[structopt(long)]
        show_sub: bool,
    },
    Template {
        #[structopt(long, short)]
        edit: bool,
        #[structopt(help = TYPE_HELP)]
        ty: ScriptFullType,
    },
}

impl Types {
    pub fn sanitize(&mut self) {
        match self.subcmd.as_ref() {
            None => self.subcmd = Some(TypesSubs::LS { show_sub: false }),
            Some(TypesSubs::Other(args)) => {
                let args = ["types", "template"]
                    .into_iter()
                    .chain(args.iter().map(|s| s.as_str()));
                self.subcmd = Some(TypesSubs::from_iter(args));
            }
            _ => (),
        }
    }
}
