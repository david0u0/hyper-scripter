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
    #[structopt(external_subcommand)]
    Other(Vec<String>),
    LS,
    Template {
        #[structopt(long, short)]
        edit: bool,
        ty: ScriptType,
    },
}

impl Types {
    pub fn sanitize(&mut self) {
        match self.subcmd.as_ref() {
            None => self.subcmd = Some(TypesSubs::LS),
            Some(TypesSubs::Other(args)) => {
                let args = std::array::IntoIter::new(["types", "template"])
                    .chain(args.iter().map(|s| s.as_str()));
                self.subcmd = Some(TypesSubs::from_iter(args));
            }
            _ => (),
        }
    }
}
