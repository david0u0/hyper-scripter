use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use colored::Color;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::PathBuf;

pub const ANONYMOUS: &'static str = ".anonymous";

#[derive(Debug)]
pub struct ScriptMeta {
    pub name: ScriptName,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash, Ord)]
pub enum ScriptName {
    Anonymous(u32),
    Named(String),
}
impl std::fmt::Display for ScriptName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptName::Anonymous(id) => write!(f, ".{}", id),
            ScriptName::Named(name) => write!(f, "{}", name),
        }
    }
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
        log::debug!("解析腳本名：{}", self);
        let reg = regex::Regex::new(r"^\.(\w+)$")?;
        let m = reg.captures(&self);
        if let Some(m) = m {
            let id_str = m
                .get(1)
                .ok_or(Error::ScriptNameFormat(self.clone()))?
                .as_str();
            match id_str.parse::<u32>() {
                Ok(id) => Ok(ScriptName::Anonymous(id)),
                Err(e) => return Err(Error::ScriptNameFormat(self.clone()).context(e)),
            }
        } else {
            if self.find(".").is_some() || self.find(" ").is_some() {
                return Err(Error::ScriptNameFormat(self.clone()).context("解析命名腳本失敗"));
            }
            Ok(ScriptName::Named(self))
        }
    }
}
impl<'a> ToScriptName for &'a ScriptName {
    fn to_script_name(self) -> Result<ScriptName> {
        Ok(self.clone())
    }
}
impl ToScriptName for ScriptName {
    fn to_script_name(self) -> Result<ScriptName> {
        Ok(self)
    }
}
impl ScriptName {
    pub fn to_file_name(&self, ty: ScriptType) -> String {
        let ext = ty.ext().map(|s| format!(".{}", s)).unwrap_or_default();
        match self {
            ScriptName::Anonymous(id) => format!("{}/{}{}", ANONYMOUS, id, ext),
            ScriptName::Named(name) => format!("{}{}", name, ext),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScriptInfo {
    pub edit_time: DateTime<Utc>,
    pub exec_time: Option<DateTime<Utc>>,
    pub hidden: bool,
    pub name: ScriptName,
    pub ty: ScriptType,
}

impl ScriptInfo {
    pub fn last_time(&self) -> DateTime<Utc> {
        if let Some(exec_time) = self.exec_time {
            std::cmp::max(self.edit_time, exec_time)
        } else {
            self.edit_time
        }
    }
    pub fn file_name(&self) -> String {
        self.name.to_file_name(self.ty)
    }
    pub fn new(name: ScriptName, ty: ScriptType) -> Result<Self> {
        Ok(ScriptInfo {
            name,
            ty,
            edit_time: Utc::now(),
            exec_time: None,
            hidden: false,
        })
    }
}

macro_rules! script_type_enum {
    ($( [$tag:expr, $color:expr] => $name:ident$(($ext:expr))?: ( $($args:expr),* ) ),*) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
        pub enum ScriptType {
            $($name),*
        }
        #[allow(unreachable_code)]
        impl ScriptType {
            pub fn ext(&self) -> Option<&'static str> {
                match self {
                    $(
                        ScriptType::$name => {
                            $(return Some($ext);)?
                            None
                        }
                    )*
                }
            }
            pub fn color(&self) -> Color {
                match self {
                    $(
                        ScriptType::$name => {
                            $color
                        }
                    )*
                }
            }
            pub fn cmd(&self) -> Option<(String, Vec<String>)> {
                match self {
                    $(
                        ScriptType::$name => {
                            let v: &[&str] = &[$($args),*];
                            if v.len() > 0 {
                                Some(
                                    (
                                        v[0].to_string(),
                                        v[1..v.len()].into_iter().map(|s| s.to_string()).collect()
                                    )
                                )
                            } else {
                                None
                            }
                        }
                    )*
                }
            }
        }
        impl std::str::FromStr for ScriptType {
            type Err = String;
            fn from_str(s: &str) -> std::result::Result<Self, String> {
                match s {
                    $(
                        $tag => {
                            Ok(ScriptType::$name)
                        }
                    )*
                    _ => {
                        let v = &[$($tag),*];
                        let expected = v.join("/").to_string();
                        Err(format!("ScriptMeta type expected {}, get {}", expected, s))
                    }
                }
            }
        }
        impl std::fmt::Display for ScriptType {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                        ScriptType::$name => {
                            write!(f, $tag)?;
                        }
                    )*
                }
                Ok(())
            }
        }
    };
}

script_type_enum! {
    ["sh", Color::Green] => Shell("sh"): ("bash"),
    ["screen", Color::White] => Screen: ("screen", "-c"),
    ["txt", Color::BrightBlack] => Txt: (),
    ["js", Color::BrightCyan] => Js("js"): ("node"),
    ["rb", Color::BrightRed] => Rb("rb"): ("ruby")
}
impl Default for ScriptType {
    fn default() -> Self {
        ScriptType::Shell
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_ext() {
        assert_eq!(Some("sh"), ScriptType::Shell.ext());
        assert_eq!(None, ScriptType::Screen.ext());
    }
    #[test]
    fn test_cmd() {
        assert_eq!(Some(("node".to_owned(), vec![])), ScriptType::Js.cmd());
        assert_eq!(None, ScriptType::Txt.cmd());
    }
}
