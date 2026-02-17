use super::help_str::*;
use crate::script_type::ScriptFullType;
use clap::{Error as ClapError, Parser};
use serde::Serialize;

#[derive(Parser, Debug, Serialize)]
pub struct Types {
    #[command(subcommand)]
    pub subcmd: Option<TypesSubs>,
}

#[derive(Parser, Debug, Serialize)]
#[command(allow_hyphen_values = true)] // 為了允許 hs types --edit sh 這樣的命令
pub enum TypesSubs {
    #[command(external_subcommand)]
    Other(Vec<String>),
    LS {
        #[arg(long)]
        no_sub: bool,
    },
    Template {
        #[arg(long, short)]
        edit: bool,
        #[arg(help = TYPE_HELP)]
        ty: ScriptFullType,
    },
}

impl Types {
    pub fn sanitize(&mut self) -> Result<(), ClapError> {
        match self.subcmd.as_ref() {
            None => self.subcmd = Some(TypesSubs::LS { no_sub: false }),
            Some(TypesSubs::Other(args)) => {
                let args = ["types", "template"]
                    .into_iter()
                    .chain(args.iter().map(|s| s.as_str()));
                self.subcmd = Some(TypesSubs::try_parse_from(args)?);
            }
            _ => (),
        }
        Ok(())
    }
}
