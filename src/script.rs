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
        log::trace!("解析腳本名：{}", self);
        let reg = regex::Regex::new(r"^\.(\w+)$")?;
        let m = reg.captures(&self);
        if let Some(m) = m {
            let id_str = m
                .get(1)
                .ok_or(Error::ScriptNameFormat(self.clone()))?
                .as_str();
            match id_str.parse::<u32>() {
                Ok(id) => Ok(ScriptName::Anonymous(id)),
                _ => return Err(Error::ScriptNameFormat(self.clone())),
            }
        } else {
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
    pub fn to_file_name(&self, ty: CommandType) -> String {
        let ext = ty.ext().map(|s| format!(".{}", s)).unwrap_or_default();
        match self {
            ScriptName::Anonymous(id) => format!("{}/{}{}", ANONYMOUS, id, ext),
            ScriptName::Named(name) => format!("{}{}", name, ext),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScriptInfo {
    pub edit_time: DateTime<Utc>,
    pub exec_time: Option<DateTime<Utc>>,
    pub hidden: bool,
    pub name: ScriptName,
    pub ty: CommandType,
    pub last_edit_path: PathBuf,
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
    pub fn new(name: ScriptName, ty: CommandType) -> Result<Self> {
        Ok(ScriptInfo {
            name,
            ty,
            last_edit_path: std::env::current_dir()?,
            edit_time: Utc::now(),
            exec_time: None,
            hidden: false,
        })
    }
}

macro_rules! script_type_enum {
    ($( [$tag:expr, $color:expr] => $name:ident$(($ext:expr))?: ( $($args:expr),* ) ),*) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
        pub enum CommandType {
            $($name),*
        }
        #[allow(unreachable_code)]
        impl CommandType {
            pub fn ext(&self) -> Option<&'static str> {
                match self {
                    $(
                        CommandType::$name => {
                            $(return Some($ext);)?
                            None
                        }
                    )*
                }
            }
            pub fn color(&self) -> Color {
                match self {
                    $(
                        CommandType::$name => {
                            $color
                        }
                    )*
                }
            }
            pub fn cmd(&self) -> Option<(String, Vec<String>)> {
                match self {
                    $(
                        CommandType::$name => {
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
        impl std::str::FromStr for CommandType {
            type Err = String;
            fn from_str(s: &str) -> std::result::Result<Self, String> {
                match s {
                    $(
                        $tag => {
                            Ok(CommandType::$name)
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
        impl std::fmt::Display for CommandType {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                        CommandType::$name => {
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
impl Default for CommandType {
    fn default() -> Self {
        CommandType::Shell
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_ext() {
        assert_eq!(Some("sh"), CommandType::Shell.ext());
        assert_eq!(None, CommandType::Screen.ext());
    }
    #[test]
    fn test_cmd() {
        assert_eq!(Some(("node".to_owned(), vec![])), CommandType::Js.cmd());
        assert_eq!(None, CommandType::Txt.cmd());
    }
}
