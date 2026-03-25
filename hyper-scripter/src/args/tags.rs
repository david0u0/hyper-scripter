use crate::tag::TagSelector;
use clap::{Error as ClapError, Parser};
use serde::Serialize;
use supplement::Supplement;

#[derive(Parser, Debug, Serialize, Supplement)]
#[command(allow_hyphen_values = true)]
pub enum Tags {
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
        match self {
            Tags::Other(args) => {
                let args = ["tags", "set"]
                    .into_iter()
                    .chain(args.iter().map(|s| s.as_str()));
                *self = Tags::try_parse_from(args)?;
            }
            _ => (),
        }
        Ok(())
    }
}
