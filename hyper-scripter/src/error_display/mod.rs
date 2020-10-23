use crate::error::{Error, Error::*, FormatCode, SysPath};
use std::fmt::{Display, Formatter, Result};

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Empty => write!(f, "No existing script!")?,
            SysPathNotFound(SysPath::Config) => write!(
                f,
                "Can not find you're config path. Usually it should be `$HOME/.config`",
            )?,
            SysPathNotFound(SysPath::Home) => write!(f, "Can not find you're home path.")?,
            PermissionDenied(v) => {
                write!(f, "Permission denied")?;
                if v.len() > 0 {
                    writeln!(f, ":")?;
                }
                for p in v.iter() {
                    writeln!(f, "{}", p.to_string_lossy())?;
                }
            }
            PathNotFound(p) => write!(f, "Path not found: {}", p.to_string_lossy())?,
            ScriptExist(name) => write!(f, "Script already exist: {}", name)?,
            ScriptNotFound(name) => write!(f, "Script not found: {}", name)?,
            CategoryMismatch { expect, actual } => write!(
                f,
                "Script Category mismatch. Expect: {}, Actual: {}",
                expect, actual
            )?,
            MultiFuzz(v) => {
                writeln!(f, "Multiple scripts with same fuzzy score:")?;
                for name in v {
                    writeln!(f, "{}", name)?;
                }
            }
            UnknownCategory(c) => write!(f, "Unknown category: {}", c)?,
            Format(code, s) => {
                write!(f, "Format error for ")?;
                use FormatCode::*;
                match code {
                    Config => write!(f, "config file")?,
                    ScriptName => write!(f, "script name")?,
                    Regex => write!(f, "regular expression")?,
                    ScriptQuery => write!(f, "script query")?,
                    Tag => write!(f, "tag")?,
                    FilterQuery => write!(f, "tag filter")?,
                }
                write!(f, " '{}'", s)?;
            }
            ScriptError(code) => write!(f, "Script exited unexpectedly with {}", code)?,
            NoAlias(alias) => write!(f, "No such alias: {}", alias)?,
            _ => {
                log::warn!("未被正確打印的錯誤：{:?}", self);
                write!(f, "{:?}", self)?;
            }
        }
        Ok(())
    }
}
