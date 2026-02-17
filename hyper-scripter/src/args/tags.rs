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
    LS {
        #[arg(long, short)]
        known: bool,
        #[arg(long, short, conflicts_with = "known")]
        named: bool,
    },
    Toggle {
        names: Vec<String>,
    },
}

impl Tags {
    pub fn sanitize(&mut self) -> Result<(), ClapError> {
        match self.subcmd.as_ref() {
            None => {
                self.subcmd = Some(TagsSubs::LS {
                    named: false,
                    known: false,
                })
            }
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
