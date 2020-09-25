use crate::error::{
    Contextable, Error, FormatCode::FilterQuery as FilterQueryCode,
    FormatCode::ScriptQuery as ScriptQueryCode, Result,
};
use crate::script::{AsScriptName, ScriptName};
use crate::tag::TagControlFlow;
use std::str::FromStr;

#[derive(Debug)]
pub enum EditQuery {
    NewAnonimous,
    Query(ScriptQuery),
}
impl FromStr for EditQuery {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Ok(if s == "." {
            EditQuery::NewAnonimous
        } else {
            EditQuery::Query(ScriptQuery::from_str(s)?)
        })
    }
}
#[derive(Debug)]
pub enum ScriptQuery {
    Fuzz(String),
    Exact(ScriptName<'static>),
    Prev(usize),
}
impl AsScriptName for ScriptQuery {
    fn as_script_name(&self) -> Result<ScriptName> {
        match self {
            ScriptQuery::Fuzz(s) => s.as_script_name(),
            ScriptQuery::Exact(name) => name.as_script_name(),
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
        if s.starts_with('=') {
            s = &s[1..s.len()];
            let name = s.as_script_name()?.into_static();
            Ok(ScriptQuery::Exact(name))
        } else if s == "-" {
            Ok(ScriptQuery::Prev(1))
        } else if s.starts_with('^') {
            Ok(ScriptQuery::Prev(parse_prev(s)?))
        } else {
            s.as_script_name().context("模糊搜尋仍需符合腳本名格式！")?; // NOTE: 單純檢查用
            Ok(ScriptQuery::Fuzz(s.to_owned()))
        }
    }
}

#[derive(Debug)]
pub struct FilterQuery {
    pub name: Option<String>,
    pub content: TagControlFlow,
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
                    content: FromStr::from_str(s)?,
                })
            }
            &[name, s] => {
                log::trace!("解析有名篩選器：{} = {}", name, s);
                Ok(FilterQuery {
                    // TODO: 檢查名字
                    name: Some(name.to_owned()),
                    content: FromStr::from_str(s)?,
                })
            }
            _ => Err(Error::Format(FilterQueryCode, s.to_owned())),
        }
    }
}
