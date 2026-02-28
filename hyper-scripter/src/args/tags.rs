use crate::tag::TagSelector;
use clap::{Error as ClapError, Parser};
use serde::Serialize;

#[derive(Parser, Debug, Serialize)]
pub struct Tags {
    #[command(subcommand)]
    pub subcmd: Option<TagsSubs>,
}

#[derive(Parser, Debug, Serialize)]
#[command(allow_hyphen_values = true)]
pub enum TagsSubs {
    #[command(external_subcommand)]
    Other(Vec<String>),
    Unset {
        name: String,
    }, // TODO: new type?
    Set {
        #[arg(long, short)]
        name: Option<String>,
        content: TagSelector,
    },
    Toggle {
        names: Vec<String>,
    },
}

impl Tags {
    pub fn sanitize(&mut self) -> Result<(), ClapError> {
        match self.subcmd.as_ref() {
            Some(TagsSubs::Other(args)) => {
                let args = ["tags", "set"]
                    .into_iter()
                    .chain(args.iter().map(|s| s.as_str()));
                self.subcmd = Some(TagsSubs::try_parse_from(args)?);
            }
            _ => (),
        }
        Ok(())
    }
}
