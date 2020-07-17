use crate::error::{Error, Result};
use regex::Regex;
use std::path::PathBuf;
#[derive(Debug)]
pub struct Script {
    pub name: ScriptName,
    pub path: PathBuf,
    pub exist: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ScriptName {
    Anonymous(u32),
    Named(String),
}
impl ScriptName {
    pub fn parse(name: String) -> Result<Self> {
        let reg = Regex::new(r"^\.(\d+)$")?;
        let m = reg.captures(&name);
        if let Some(m) = m {
            let id_str = m.get(1).ok_or(Error::Format(name.clone()))?.as_str();
            match id_str.parse::<u32>() {
                Ok(id) => Ok(ScriptName::Anonymous(id)),
                _ => return Err(Error::Format(name)),
            }
        } else {
            Ok(ScriptName::Named(name))
        }
    }
    pub fn to_cmd(&self) -> String {
        match self {
            ScriptName::Anonymous(id) => format!("{}.sh", id),
            ScriptName::Named(name) => format!("{}.sh", name),
        }
    }
}
