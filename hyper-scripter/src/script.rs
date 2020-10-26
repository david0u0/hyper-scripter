use crate::config::Config;
use crate::error::{Contextable, Error, FormatCode::ScriptName as ScriptNameCode, Result};
use crate::fuzzy::FuzzKey;
use crate::script_time::ScriptTime;
use crate::script_type::ScriptType;
use crate::tag::Tag;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::PathBuf;

pub const ANONYMOUS: &'static str = ".anonymous";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Ord)]
pub enum ScriptName<'a> {
    Anonymous(u32),
    Named(Cow<'a, str>),
}
impl ScriptName<'_> {
    pub fn namespaces(&self) -> Vec<&'_ str> {
        match self {
            ScriptName::Anonymous(_) => vec![],
            ScriptName::Named(s) => {
                let mut v: Vec<_> = s.split("/").collect();
                v.pop();
                v
            }
        }
    }
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
    pub exec_time: Option<ScriptTime<String>>,
    pub miss_time: Option<ScriptTime>,
    pub id: i64,
    pub name: ScriptName<'a>,
    pub tags: HashSet<Tag>,
    pub ty: ScriptType,
}
impl FuzzKey for ScriptInfo<'_> {
    fn fuzz_key(&self) -> Cow<'_, str> {
        self.name.fuzz_key()
    }
}

impl<'a> ScriptInfo<'a> {
    pub fn cp(&self, new_name: ScriptName) -> Self {
        let now = ScriptTime::now(());
        ScriptInfo {
            name: new_name.into_static(),
            read_time: now.clone(),
            write_time: now.clone(),
            created_time: now,
            exec_time: None,
            ..self.clone()
        }
    }
    pub fn last_time(&self) -> NaiveDateTime {
        use std::cmp::max;
        fn map<T: Clone + std::fmt::Debug>(time: &Option<ScriptTime<T>>) -> NaiveDateTime {
            match time {
                Some(time) => **time,
                _ => NaiveDateTime::from_timestamp(1, 0),
            }
        }
        max(
            *self.read_time,
            max(map(&self.exec_time), map(&self.miss_time)),
        )
    }
    pub fn file_path(&self) -> Result<PathBuf> {
        self.name.to_file_path(&self.ty)
    }
    pub fn read(&mut self) {
        self.read_time = ScriptTime::now(());
    }
    pub fn write(&mut self) {
        let now = ScriptTime::now(());
        self.read_time = now.clone();
        self.write_time = now;
    }
    pub fn exec(&mut self, content: String) {
        log::trace!("{:?} 執行內容為 {}", self, content);
        self.exec_time = Some(ScriptTime::now(content));
        self.read_time = ScriptTime::now(());
    }
    pub fn builder<'b>(
        id: i64,
        name: ScriptName<'b>,
        ty: ScriptType,
        tags: impl Iterator<Item = Tag>,
    ) -> ScriptBuilder<'b> {
        ScriptBuilder {
            id,
            name,
            ty,
            tags: tags.collect(),
            read_time: None,
            created_time: None,
            exec_time: None,
            miss_time: None,
            write_time: None,
        }
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
                || self.starts_with(".")
                || self.find("..").is_some()
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

#[derive(Debug)]
pub struct ScriptBuilder<'a> {
    pub name: ScriptName<'a>,
    read_time: Option<NaiveDateTime>,
    created_time: Option<NaiveDateTime>,
    write_time: Option<NaiveDateTime>,
    miss_time: Option<NaiveDateTime>,
    exec_time: Option<NaiveDateTime>,
    id: i64,
    tags: HashSet<Tag>,
    ty: ScriptType,
}

impl<'a> ScriptBuilder<'a> {
    pub fn exec_time(mut self, time: NaiveDateTime) -> Self {
        self.exec_time = Some(time);
        self
    }
    pub fn miss_time(mut self, time: NaiveDateTime) -> Self {
        self.miss_time = Some(time);
        self
    }
    pub fn read_time(mut self, time: NaiveDateTime) -> Self {
        self.read_time = Some(time);
        self
    }
    pub fn write_time(mut self, time: NaiveDateTime) -> Self {
        self.write_time = Some(time);
        self
    }
    pub fn created_time(mut self, time: NaiveDateTime) -> Self {
        self.created_time = Some(time);
        self
    }
    pub fn build(self) -> ScriptInfo<'a> {
        let now = ScriptTime::now(());
        let created_time = ScriptTime::new_or(self.created_time, now);
        ScriptInfo {
            id: self.id,
            name: self.name,
            ty: self.ty,
            tags: self.tags,
            write_time: ScriptTime::new_or(self.write_time, created_time.clone()),
            read_time: ScriptTime::new_or(self.read_time, created_time.clone()),
            created_time,
            exec_time: self.exec_time.map(|t| ScriptTime::new(t)),
            miss_time: self.miss_time.map(|t| ScriptTime::new(t)),
        }
    }
}
