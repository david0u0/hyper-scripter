use crate::error::{
    Contextable, Error, FormatCode::Regex as RegexCode, FormatCode::ScriptQuery as ScriptQueryCode,
    Result,
};
use crate::impl_ser_by_to_string;
use crate::script::{IntoScriptName, ScriptName};
use regex::Regex;
use serde::Serialize;
use std::str::FromStr;

mod util;
pub use util::*;
mod range_query;
pub use range_query::*;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub enum EditQuery<Q> {
    NewAnonimous,
    Query(Q),
}
impl<Q: FromStr<Err = Error>> FromStr for EditQuery<Q> {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Ok(if s == "?" {
            EditQuery::NewAnonimous
        } else {
            EditQuery::Query(s.parse()?)
        })
    }
}

#[derive(Debug)]
pub enum ListQuery {
    Pattern(Regex, String),
    Query(ScriptQuery),
}
impl FromStr for ListQuery {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        if s.contains('*') {
            let s = s.to_owned();
            // TODO: 好好檢查
            let re = s.replace(".", r"\.");
            let re = re.replace("*", ".*");
            match Regex::new(&format!("^{}$", re)) {
                Ok(re) => Ok(ListQuery::Pattern(re, s)),
                Err(e) => {
                    log::error!("正規表達式錯誤：{}", e);
                    Err(Error::Format(RegexCode, s))
                }
            }
        } else {
            Ok(ListQuery::Query(s.parse()?))
        }
    }
}
impl std::fmt::Display for ListQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            ListQuery::Query(q) => write!(f, "{}", q),
            ListQuery::Pattern(_, s) => write!(f, "{}", s),
        }?;
        Ok(())
    }
}
impl_ser_by_to_string!(ListQuery);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScriptQuery {
    inner: ScriptQueryInner,
    bang: bool,
}
impl Default for ScriptQuery {
    fn default() -> Self {
        ScriptQuery {
            inner: ScriptQueryInner::Prev(1),
            bang: false,
        }
    }
}
impl std::fmt::Display for ScriptQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            ScriptQueryInner::Fuzz(fuzz) => write!(f, "{}", fuzz),
            ScriptQueryInner::Exact(e) => write!(f, "={}", e),
            ScriptQueryInner::Prev(p) => write!(f, "^{}", p),
        }?;
        if self.bang {
            write!(f, "!")?;
        }
        Ok(())
    }
}
impl_ser_by_to_string!(ScriptQuery);

#[derive(Debug, Clone, Eq, PartialEq)]
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
        let bang = if s.ends_with('!') {
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
            ScriptName::valid(s, true, true, true).context("模糊搜尋仍需符合腳本名格式！")?; // NOTE: 單純檢查用
            ScriptQueryInner::Fuzz(s.to_owned())
        };
        Ok(ScriptQuery { inner, bang })
    }
}
