use crate::error::Error;
use crate::script::{AsScriptName, ScriptName};
use std::str::FromStr;

#[derive(Debug)]
pub enum ScriptArg {
    Fuzz(String),
    Exact(ScriptName<'static>),
    Prev(u32),
}
impl AsScriptName for ScriptArg {
    fn as_script_name(&self) -> Result<ScriptName, Error> {
        match self {
            ScriptArg::Fuzz(s) => s.as_script_name(),
            ScriptArg::Exact(name) => name.as_script_name(),
            _ => panic!("歷史查詢沒有名字"),
        }
    }
}

impl FromStr for ScriptArg {
    type Err = Error;
    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("=") {
            s = &s[1..s.len()];
            let name = s.as_script_name()?.into_static();
            Ok(ScriptArg::Exact(name))
        } else if s == "-" {
            Ok(ScriptArg::Prev(1))
        } else if s.starts_with("-") {
            // TODO:
            match s[1..s.len()].parse::<u32>() {
                Ok(prev) => Ok(ScriptArg::Prev(prev)),
                Err(e) => {
                    log::error!("解析整數錯誤：{}", e);
                    Err(Error::Format(s.to_owned()))
                }
            }
        } else {
            s.as_script_name()?; // NOTE: 單純檢查用
            Ok(ScriptArg::Fuzz(s.to_owned()))
        }
    }
}
