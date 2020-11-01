use crate::error::{Error, Error::*, FormatCode, SysPath};
use std::fmt::{Display, Formatter, Result};
use std::path::PathBuf;

fn fmt_multi_path(f: &mut Formatter, msg: &str, mutli_path: &[PathBuf]) -> Result {
    write!(f, "{}", msg)?;
    if mutli_path.len() > 0 {
        write!(f, ":")?;
    }
    let mut it = mutli_path.iter();
    if let Some(p) = it.next() {
        writeln!(f, "")?;
        write!(f, "{}", p.to_string_lossy())?;
    }
    while let Some(p) = it.next() {
        write!(f, "{}", p.to_string_lossy())?;
    }
    Ok(())
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        // LOCALE
        match self {
            DontFuzz => return Ok(()),
            Empty => write!(f, "No existing script!")?,
            SysPathNotFound(SysPath::Config) => write!(
                f,
                "Can not find you're config path. Usually it should be `$HOME/.config`",
            )?,
            SysPathNotFound(SysPath::Home) => write!(f, "Can not find you're home path.")?,
            PermissionDenied(v) => fmt_multi_path(f, "Permission denied", v)?,
            PathNotFound(v) => fmt_multi_path(f, "Path not found", v)?,
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
        writeln!(f, "")
    }
}
