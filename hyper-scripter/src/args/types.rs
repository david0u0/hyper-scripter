use super::help_str::*;
use crate::script_type::ScriptFullType;
use clap::Parser;
use serde::Serialize;
use supplement::Supplement;

#[derive(Parser, Debug, Serialize, Supplement)]
pub struct Types {
    #[arg(long, conflicts_with_all = &["ty", "edit"])]
    pub no_sub: bool,
    #[arg(long, short, requires = "ty")]
    pub edit: bool,
    #[arg(help = TYPE_HELP)]
    pub ty: Option<ScriptFullType>,
}
