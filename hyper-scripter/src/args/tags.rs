use crate::tag::TagSelector;
use clap::{Error as ClapError, Parser};
use serde::Serialize;

#[derive(Parser, Debug, Serialize)]
pub struct Tags {
    #[clap(subcommand)]
    pub subcmd: Option<TagsSubs>,
}

#[derive(Parser, Debug, Serialize)]
#[clap(allow_hyphen_values = true)]
pub enum TagsSubs {
    #[clap(external_subcommand)]
    Other(Vec<String>),
    Unset {
        name: String,
    }, // TODO: new type?
    Set {
        #[clap(long, short)]
        name: Option<String>,
        content: TagSelector,
    },
    LS {
        #[clap(long, short)]
        known: bool,
        #[clap(long, short, conflicts_with = "known")]
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
