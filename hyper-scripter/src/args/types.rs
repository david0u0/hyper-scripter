use super::help_str::*;
use crate::script_type::ScriptFullType;
use clap::Parser;
use serde::Serialize;

#[derive(Parser, Debug, Serialize)]
pub struct Types {
    #[clap(subcommand)]
    pub subcmd: Option<TypesSubs>,
}

#[derive(Parser, Debug, Serialize)]
#[clap(allow_hyphen_values = true)] // 為了允許 hs types --edit sh 這樣的命令
pub enum TypesSubs {
    #[clap(external_subcommand)]
    Other(Vec<String>),
    LS {
        #[clap(long)]
        no_sub: bool,
    },
    Template {
        #[clap(long, short)]
        edit: bool,
        #[clap(help = TYPE_HELP)]
        ty: ScriptFullType,
    },
}

impl Types {
    pub fn sanitize(&mut self) {
        match self.subcmd.as_ref() {
            None => self.subcmd = Some(TypesSubs::LS { no_sub: false }),
            Some(TypesSubs::Other(args)) => {
                let args = ["types", "template"]
                    .into_iter()
                    .chain(args.iter().map(|s| s.as_str()));
                self.subcmd = Some(TypesSubs::parse_from(args));
            }
            _ => (),
        }
    }
}
