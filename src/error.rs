use crate::script_type::ScriptType;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub enum SysPath {
    Config,
    Home,
}
#[derive(Debug, Clone, Copy)]
pub enum FormatCode {
    Config,
    ScriptName,
    Regex,
    ScriptQuery,
    Tag,
}
#[derive(Debug, Clone)]
pub enum Error {
    Others(
        Vec<String>,
        Option<Arc<dyn 'static + Send + Sync + std::error::Error>>,
    ),
    SysPathNotFound(SysPath),

    PermissionDenied(Vec<PathBuf>),
    // NOTE: PathNotFound 比 ScriptNotFound 更嚴重，代表歷史記錄中有這支腳本，實際要找卻找不到
    PathNotFound(PathBuf),
    GeneralFS(Vec<PathBuf>, Arc<std::io::Error>),

    ScriptExist(String),
    ScriptNotFound(String),
    CategoryMismatch {
        expect: ScriptType,
        actual: ScriptType,
    },
    MultiFuzz(Vec<String>),
    UnknownCategory(String),
    Format(FormatCode, String),
    ScriptError(i32),
    Empty,
}

impl<T: 'static + Send + Sync + std::error::Error> From<T> for Error {
    fn from(t: T) -> Self {
        Error::Others(vec![], Some(Arc::new(t)))
    }
}
impl Error {
    pub fn msg<T: ToString>(s: T) -> Self {
        Error::Others(vec![s.to_string()], None)
    }
    pub fn context<T: ToString>(mut self, s: T) -> Self {
        log::debug!("附註：{:?} + {}", self, s.to_string());
        match &mut self {
            Error::Others(msg, ..) => msg.push(s.to_string()),
            _ => (),
        }
        self
    }
}

pub type Result<T = ()> = std::result::Result<T, Error>;

pub trait Contextable<T> {
    fn context<S: ToString>(self, s: S) -> Result<T>;
}
impl<T> Contextable<T> for Result<T> {
    fn context<S: ToString>(self, s: S) -> Result<T> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(e.context(s)),
        }
    }
}

impl<T, E: 'static + Send + Sync + std::error::Error> Contextable<T> for std::result::Result<T, E> {
    fn context<S: ToString>(self, s: S) -> Result<T> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => {
                let e: Error = e.into();
                Err(e.context(s))
            }
        }
    }
}
