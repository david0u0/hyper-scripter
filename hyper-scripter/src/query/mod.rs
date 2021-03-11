use crate::error::{
    Contextable, Error, FormatCode::FilterQuery as FilterQueryCode, FormatCode::Regex as RegexCode,
    FormatCode::ScriptQuery as ScriptQueryCode, Result,
};
use crate::script::{IntoScriptName, ScriptName};
use crate::tag::TagFilter;
use regex::Regex;
use std::str::FromStr;

mod util;
pub use util::*;

#[derive(Debug)]
pub enum EditQuery {
    NewAnonimous,
    Query(ScriptQuery),
}
impl Default for EditQuery {
    fn default() -> Self {
        EditQuery::Query(ScriptQuery {
            inner: ScriptQueryInner::Prev(1),
            bang: false,
        })
    }
}
impl FromStr for EditQuery {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Ok(if s == "." {
            EditQuery::NewAnonimous
        } else {
            EditQuery::Query(s.parse()?)
        })
    }
}
#[derive(Debug)]
pub enum ListQuery {
    Pattern(Regex),
    Query(ScriptQuery),
}
impl FromStr for ListQuery {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        if s.find("*").is_some() {
            // TODO: 好好檢查
            let s = s.replace(".", r"\.");
            let s = s.replace("*", ".*");
            let re = Regex::new(&format!("^{}$", s)).map_err(|e| {
                log::error!("正規表達式錯誤：{}", e);
                Error::Format(RegexCode, s)
            })?;
            Ok(ListQuery::Pattern(re))
        } else {
            Ok(ListQuery::Query(s.parse()?))
        }
    }
}
#[derive(Debug, Clone)]
pub struct ScriptQuery {
    inner: ScriptQueryInner,
    bang: bool,
}
#[derive(Debug, Clone)]
enum ScriptQueryInner {
    Fuzz(String),
    Exact(ScriptName),
    Prev(usize),
}
impl IntoScriptName for ScriptQuery {
    fn into_script_name(self) -> Result<ScriptName> {
        match self.inner {
            ScriptQueryInner::Fuzz(s) => s.into_script_name(),
            ScriptQueryInner::Exact(name) => Ok(name),
            _ => panic!("歷史查詢沒有名字"),
        }
    }
}

fn parse_prev(s: &str) -> Result<usize> {
    // NOTE: 解析 `^^^^ = Prev(4)`
    let mut is_pure_prev = true;
    for ch in s.chars() {
        if ch != '^' {
            is_pure_prev = false;
            break;
        }
    }
    if is_pure_prev {
        return Ok(s.len());
    }
    // NOTE: 解析 `^4 = Prev(4)`
    match s[1..s.len()].parse::<usize>() {
        Ok(0) => Err(Error::Format(ScriptQueryCode, s.to_owned())).context("歷史查詢不可為0"),
        Ok(prev) => Ok(prev),
        Err(e) => Err(Error::Format(ScriptQueryCode, s.to_owned()))
            .context(format!("解析整數錯誤：{}", e)),
    }
}
impl FromStr for ScriptQuery {
    type Err = Error;
    fn from_str(mut s: &str) -> Result<Self> {
        let bang = if s.ends_with("!") {
            if s == "!" {
                return Ok(ScriptQuery {
                    inner: ScriptQueryInner::Prev(1),
                    bang: true,
                });
            }
            s = &s[..s.len() - 1];
            true
        } else {
            false
        };
        let inner = if s.starts_with('=') {
            s = &s[1..s.len()];
            let name = s.to_owned().into_script_name()?;
            ScriptQueryInner::Exact(name)
        } else if s == "-" {
            ScriptQueryInner::Prev(1)
        } else if s.starts_with('^') {
            ScriptQueryInner::Prev(parse_prev(s)?)
        } else {
            ScriptName::valid(s).context("模糊搜尋仍需符合腳本名格式！")?; // NOTE: 單純檢查用
            ScriptQueryInner::Fuzz(s.to_owned())
        };
        Ok(ScriptQuery { inner, bang })
    }
}

#[derive(Debug)]
pub struct FilterQuery {
    pub name: Option<String>,
    pub content: TagFilter,
}

impl FromStr for FilterQuery {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        let arr: Vec<&str> = s.split("=").collect();
        match AsRef::<[&str]>::as_ref(&arr) {
            &[s] => {
                log::trace!("解析無名篩選器：{}", s);
                Ok(FilterQuery {
                    name: None,
                    content: s.parse()?,
                })
            }
            &[name, s] => {
                log::trace!("解析有名篩選器：{} = {}", name, s);
                let content: TagFilter = if s.len() == 0 {
                    Default::default()
                } else {
                    s.parse()?
                };
                Ok(FilterQuery {
                    // TODO: 檢查名字
                    name: Some(name.to_owned()),
                    content,
                })
            }
            _ => Err(Error::Format(FilterQueryCode, s.to_owned())),
        }
    }
}
