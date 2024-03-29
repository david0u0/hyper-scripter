use crate::error::{Error, Error::*, FormatCode, SysPath};
use std::fmt::{Display, Formatter, Result};
use std::path::PathBuf;

fn fmt_multi_path(f: &mut Formatter, msg: &str, mutli_path: &[PathBuf]) -> Result {
    write!(f, "{}", msg)?;
    if !mutli_path.is_empty() {
        write!(f, ":")?;
    }
    let mut it = mutli_path.iter();
    if let Some(p) = it.next() {
        writeln!(f)?;
        write!(f, "{}", p.to_string_lossy())?;
    }
    for p in it {
        write!(f, "{}", p.to_string_lossy())?;
    }
    Ok(())
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        // LOCALE
        match self {
            DontFuzz | Caution => return Ok(()),
            Empty => write!(f, "No existing script!")?,
            NoPreviousArgs => write!(f, "No previous argument!")?,
            SysPathNotFound(SysPath::Config) => write!(
                f,
                "Can not find you're config path. Usually it should be `$HOME/.config`",
            )?,
            EmptyCreate => write!(f, "Not creating new script due to file not changing.")?,
            SysPathNotFound(SysPath::Home) => write!(f, "Can not find you're home path.")?,
            PermissionDenied(v) => fmt_multi_path(f, "Permission denied", v)?,
            PathNotFound(v) => fmt_multi_path(f, "Path not found", v)?,
            PathExist(path) => write!(f, "Path already exist: {:?}", path)?,
            ScriptExist(name) => write!(f, "Script already exist: {}", name)?,
            ScriptIsFiltered(name) => write!(f, "Script filtered out: {}", name)?,
            ScriptNotFound(name) => write!(f, "Script not found: {}", name)?,
            UnknownType(t) => write!(f, "Unknown type: {}", t)?,
            Format(code, s) => {
                write!(f, "Format error for ")?;
                use FormatCode::*;
                match code {
                    RangeQuery => write!(f, "range query")?,
                    Config => write!(f, "config file")?,
                    ScriptName => write!(f, "script name")?,
                    Regex => write!(f, "regular expression")?,
                    ScriptQuery => write!(f, "script query")?,
                    ScriptType => write!(f, "script type")?,
                    Tag => write!(f, "tag")?,
                    PromptLevel => write!(f, "prompt level")?,
                    EnvPair => write!(f, "env pair (e.g. VAR=1)")?,
                    Template => write!(f, "template")?,
                    NonEmptyArray => {
                        write!(f, "non-empty array")?;
                        return Ok(());
                    }
                }
                write!(f, " '{}'", s)?;
            }
            ScriptError(code) => write!(f, "Script exited unexpectedly with {}", code)?,
            PreRunError(code) => write!(f, "Pre-run script exited unexpectedly with {}", code)?,
            EditorError(code, cmd) => {
                let cmd = cmd.join(" ");
                write!(f, "Editor `{}` exited unexpectedly with {}", cmd, code)?
            }
            NoAlias(alias) => write!(f, "No such alias: {}", alias)?,
            RedundantOpt(opt) => write!(f, "Redundant option: {:?}", opt)?,
            _ => {
                log::warn!("未被正確打印的錯誤：{:?}", self);
                write!(f, "{:?}", self)?;
            }
        }
        writeln!(f)
    }
}
