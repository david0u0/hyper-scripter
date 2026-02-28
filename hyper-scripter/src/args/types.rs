use super::help_str::*;
use crate::script_type::ScriptFullType;
use clap::Parser;
use serde::Serialize;

#[derive(Parser, Debug, Serialize)]
pub struct Types {
    #[arg(long, conflicts_with = "ty")]
    pub no_sub: bool,
    #[arg(long, short, requires = "ty")]
    pub edit: bool,
    #[arg(help = TYPE_HELP)]
    pub ty: Option<ScriptFullType>,
}
