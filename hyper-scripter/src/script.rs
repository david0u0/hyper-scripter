use crate::config::Config;
use crate::error::{Contextable, Error, FormatCode::ScriptName as ScriptNameCode, Result};
use crate::fuzzy::FuzzKey;
use crate::script_time::ScriptTime;
use crate::script_type::ScriptType;
use crate::tag::Tag;
use chrono::{NaiveDateTime, Utc};
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
    pub fn is_anonymous(&self) -> bool {
        match self {
            ScriptName::Anonymous(_) => true,
            _ => false,
        }
    }
    pub fn key(&self) -> Cow<'_, str> {
        match self {
            ScriptName::Anonymous(id) => Cow::Owned(format!(".{}", id)),
            ScriptName::Named(s) => Cow::Borrowed(&*s),
        }
    }
    pub fn to_file_path(&self, ty: &ScriptType) -> Result<PathBuf> {
        let mut file_name: String;
        let add_ext = |name: &mut String| -> Result<()> {
            if let Some(ext) = &Config::get()?.get_script_conf(ty)?.ext {
                *name = format!("{}.{}", name, ext);
            }
            Ok(())
        };
        match self {
            ScriptName::Anonymous(id) => {
                file_name = id.to_string();
                let dir: PathBuf = ANONYMOUS.into();
                add_ext(&mut file_name)?;
                Ok(dir.join(file_name))
            }
            ScriptName::Named(name) => {
                file_name = name.to_string();
                add_ext(&mut file_name)?;
                Ok(file_name.into())
            }
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

#[derive(Debug, Clone)]
pub struct ScriptInfo<'a> {
    pub read_time: ScriptTime,
    pub created_time: ScriptTime,
    pub write_time: ScriptTime,
    pub exec_time: Option<ScriptTime>,
    pub id: i64,
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
    pub fn cp(&self, new_name: ScriptName) -> Self {
        let now = ScriptTime::now();
        ScriptInfo {
            name: new_name.into_static(),
            read_time: now,
            write_time: now,
            created_time: now,
            exec_time: None,
            ..self.clone()
        }
    }
    pub fn last_time(&self) -> NaiveDateTime {
        let time = match self.exec_time {
            Some(exec_time) => std::cmp::max(self.read_time, exec_time),
            _ => self.read_time,
        };
        *time
    }
    pub fn file_path(&self) -> Result<PathBuf> {
        self.name.to_file_path(&self.ty)
    }
    pub fn new<'a>(
        id: i64,
        name: ScriptName<'a>,
        ty: ScriptType,
        tags: impl Iterator<Item = Tag>,
        exec_time: Option<NaiveDateTime>,
        created_time: Option<NaiveDateTime>,
        write_time: Option<NaiveDateTime>,
        read_time: Option<NaiveDateTime>,
    ) -> ScriptInfo<'a> {
        let now = Utc::now().naive_local();
        ScriptInfo {
            id,
            name,
            ty,
            tags: tags.collect(),
            created_time: ScriptTime::new_or(created_time, now),
            write_time: ScriptTime::new_or(write_time, now),
            read_time: ScriptTime::new_or(read_time, now),
            exec_time: exec_time.map(|t| ScriptTime::new(t)),
        }
    }
    pub fn read(&mut self) {
        self.read_time = ScriptTime::now();
    }
    pub fn write(&mut self) {
        let now = ScriptTime::now();
        self.read_time = now;
        self.write_time = now;
    }
    pub fn exec(&mut self) {
        let now = ScriptTime::now();
        self.exec_time = Some(now);
        self.read_time = now;
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
                Err(e) => return Err(Error::Format(ScriptNameCode, self.to_owned())).context(e),
            }
        } else {
            if self.starts_with("-")
                || self.find(".").is_some()
                || self.find(" ").is_some()
                || self.len() == 0
            {
                return Err(Error::Format(ScriptNameCode, self.to_owned()))
                    .context("命名腳本格式有誤");
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
