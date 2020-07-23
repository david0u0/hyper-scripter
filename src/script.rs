use crate::error::{Error, Result};
use crate::fuzzy::FuzzKey;
use crate::tag::Tag;
use chrono::{DateTime, Utc};
use colored::Color;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::path::PathBuf;

pub const ANONYMOUS: &'static str = ".anonymous";

#[derive(Debug)]
pub struct ScriptMeta<'a> {
    pub name: ScriptName<'a>,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Ord)]
pub enum ScriptName<'a> {
    Anonymous(u32),
    Named(Cow<'a, str>),
}
impl ScriptName<'_> {
    pub fn key(&self) -> Cow<'_, str> {
        match self {
            ScriptName::Anonymous(id) => Cow::Owned(format!(".{}", id)),
            ScriptName::Named(s) => Cow::Borrowed(&*s),
        }
    }
    pub fn to_file_name(&self, ty: ScriptType) -> String {
        let ext = ty.ext().map(|s| format!(".{}", s)).unwrap_or_default();
        match self {
            ScriptName::Anonymous(id) => format!("{}/{}{}", ANONYMOUS, id, ext),
            ScriptName::Named(name) => format!("{}{}", name, ext),
        }
    }
    pub fn into_static(self) -> ScriptName<'static> {
        match self {
            ScriptName::Anonymous(id) => ScriptName::Anonymous(id),
            ScriptName::Named(name) => ScriptName::Named(Cow::Owned(name.into_owned())),
        }
    }
}
impl PartialOrd for ScriptName<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (ScriptName::Named(n1), ScriptName::Named(n2)) => n1.partial_cmp(n2),
            (ScriptName::Anonymous(i1), ScriptName::Anonymous(i2)) => i1.partial_cmp(i2),
            (ScriptName::Named(_), ScriptName::Anonymous(_)) => Some(Ordering::Less),
            (ScriptName::Anonymous(_), ScriptName::Named(_)) => Some(Ordering::Greater),
        }
    }
}
impl std::fmt::Display for ScriptName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.key())
    }
}
impl FuzzKey for ScriptName<'_> {
    fn fuzz_key(&self) -> Cow<'_, str> {
        self.key()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScriptInfo<'a> {
    pub edit_time: DateTime<Utc>,
    pub exec_time: Option<DateTime<Utc>>,
    pub name: ScriptName<'a>,
    pub tags: Vec<Tag>,
    pub ty: ScriptType,
}
impl FuzzKey for ScriptInfo<'_> {
    fn fuzz_key(&self) -> Cow<'_, str> {
        self.name.fuzz_key()
    }
}

impl ScriptInfo<'_> {
    pub fn last_time(&self) -> DateTime<Utc> {
        match self.exec_time {
            Some(exec_time) => std::cmp::max(self.edit_time, exec_time),
            _ => self.edit_time,
        }
    }
    pub fn file_name(&self) -> String {
        self.name.to_file_name(self.ty)
    }
    pub fn new<'a>(
        name: ScriptName<'a>,
        ty: ScriptType,
        tags: impl Iterator<Item = Tag>,
    ) -> Result<ScriptInfo<'a>> {
        Ok(ScriptInfo {
            name,
            ty,
            tags: tags.collect(),
            edit_time: Utc::now(),
            exec_time: None,
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

pub trait AsScriptName {
    fn as_script_name(&self) -> Result<ScriptName<'_>>;
}

impl<T: AsScriptName> AsScriptName for &T {
    fn as_script_name(&self) -> Result<ScriptName<'_>> {
        <T as AsScriptName>::as_script_name(*self)
    }
}

impl AsScriptName for str {
    fn as_script_name(&self) -> Result<ScriptName<'_>> {
        log::debug!("解析腳本名：{}", self);
        let reg = regex::Regex::new(r"^\.(\w+)$")?;
        let m = reg.captures(self);
        if let Some(m) = m {
            let id_str = m.get(1).unwrap().as_str();
            match id_str.parse::<u32>() {
                Ok(id) => Ok(ScriptName::Anonymous(id)),
                Err(e) => return Err(Error::ScriptNameFormat(self.to_owned()).context(e)),
            }
        } else {
            if self.find(".").is_some() || self.find(" ").is_some() {
                return Err(Error::ScriptNameFormat(self.to_owned()).context("命名腳本格式有誤"));
            }
            Ok(ScriptName::Named(Cow::Borrowed(self)))
        }
    }
}
impl AsScriptName for ScriptName<'_> {
    fn as_script_name(&self) -> Result<ScriptName<'_>> {
        Ok(match self {
            ScriptName::Anonymous(id) => ScriptName::Anonymous(*id),
            ScriptName::Named(s) => ScriptName::Named(Cow::Borrowed(&*s)),
        })
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
