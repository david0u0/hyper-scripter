use crate::script_type::ScriptType;
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
    LS,
    Template { ty: ScriptType },
}

impl Types {
    pub fn sanitize(&mut self) {
        match self.subcmd.as_ref() {
            None => self.subcmd = Some(TypesSubs::LS),
            _ => (),
        }
    }
}
