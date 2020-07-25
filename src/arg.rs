use crate::error::{Error, Result};
use crate::script::{AsScriptName, ScriptName};
use std::str::FromStr;

#[derive(Debug)]
pub enum ScriptArg {
    Fuzz(String),
    Exact(ScriptName<'static>),
    Prev(usize),
}
impl AsScriptName for ScriptArg {
    fn as_script_name(&self) -> Result<ScriptName> {
        match self {
            ScriptArg::Fuzz(s) => s.as_script_name(),
            ScriptArg::Exact(name) => name.as_script_name(),
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
        Ok(prev) => {
            if prev > 0 {
                return Ok(prev);
            } else {
                log::error!("歷史查詢不可為0");
            }
        }
        Err(e) => log::error!("解析整數錯誤：{}", e),
    }
    Err(Error::Format(s.to_owned()))
}
impl FromStr for ScriptArg {
    type Err = Error;
    fn from_str(mut s: &str) -> Result<Self> {
        if s.starts_with('=') {
            s = &s[1..s.len()];
            let name = s.as_script_name()?.into_static();
            Ok(ScriptArg::Exact(name))
        } else if s == "-" {
            Ok(ScriptArg::Prev(1))
        } else if s.starts_with('^') {
            Ok(ScriptArg::Prev(parse_prev(s)?))
        } else {
            s.as_script_name()?; // NOTE: 單純檢查用
            Ok(ScriptArg::Fuzz(s.to_owned()))
        }
    }
}
