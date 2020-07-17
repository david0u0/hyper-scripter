use crate::error::{Error, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug)]
pub struct Script {
    pub name: ScriptName,
    pub path: PathBuf,
    pub exist: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ScriptName {
    Anonymous(u32),
    Named(String),
}
pub trait ToScriptName {
    fn to_script_name(self) -> Result<ScriptName>;
}
impl ToScriptName for String {
    fn to_script_name(self) -> Result<ScriptName> {
        let reg = Regex::new(r"^\.(\d+)$")?;
        let m = reg.captures(&self);
        if let Some(m) = m {
            let id_str = m.get(1).ok_or(Error::Format(self.clone()))?.as_str();
            match id_str.parse::<u32>() {
                Ok(id) => Ok(ScriptName::Anonymous(id)),
                _ => return Err(Error::Format(self)),
            }
        } else {
            Ok(ScriptName::Named(self))
        }
    }
}
impl ToScriptName for ScriptName {
    fn to_script_name(self) -> Result<ScriptName> {
        Ok(self)
    }
}
impl ScriptName {
    pub fn to_cmd(&self) -> String {
        match self {
            ScriptName::Anonymous(id) => format!("{}.sh", id),
            ScriptName::Named(name) => format!("{}.sh", name),
        }
    }
}
