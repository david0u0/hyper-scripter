use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Script {
    pub name: ScriptName,
    pub path: PathBuf,
    pub exist: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash, Ord)]
pub enum ScriptName {
    Anonymous(u32),
    Named(String),
}
impl PartialOrd for ScriptName {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (ScriptName::Named(n1), ScriptName::Named(n2)) => n1.partial_cmp(n2),
            (ScriptName::Anonymous(i1), ScriptName::Anonymous(i2)) => i1.partial_cmp(i2),
            (ScriptName::Named(_), ScriptName::Anonymous(_)) => Some(Ordering::Less),
            (ScriptName::Anonymous(_), ScriptName::Named(_)) => Some(Ordering::Greater),
        }
    }
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
                _ => return Err(Error::Format(self.to_owned())),
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ScriptMeta {
    pub edit_time: DateTime<Utc>,
    pub exec_time: Option<DateTime<Utc>>,
    pub name: ScriptName,
    pub last_edit_path: PathBuf,
}

impl ScriptMeta {
    pub fn last_time(&self) -> DateTime<Utc> {
        if let Some(exec_time) = self.exec_time {
            std::cmp::max(self.edit_time, exec_time)
        } else {
            self.edit_time
        }
    }
    pub fn new(name: ScriptName) -> Result<Self> {
        Ok(ScriptMeta {
            name,
            last_edit_path: std::env::current_dir()?,
            edit_time: Utc::now(),
            exec_time: None,
        })
    }
}
