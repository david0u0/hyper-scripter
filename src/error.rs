use crate::script_type::ScriptType;
use std::path::PathBuf;
#[derive(Debug)]
pub enum Others {}
#[derive(Debug)]
pub enum Error {
    Others(
        Vec<String>,
        Option<Box<dyn 'static + Send + Sync + std::error::Error>>,
    ),
    SysPathNotFound(&'static str),

    PermissionDenied(Vec<PathBuf>),
    // NOTE: PathNotFound 比 ScriptNotFound 更嚴重，代表歷史記錄中有這支腳本，實際要找卻找不到
    PathNotFound(PathBuf),
    GeneralFS(Vec<PathBuf>, std::io::Error),

    ScriptExist(String),
    ScriptNotFound(String),
    Operation(String),
    TypeMismatch {
        expect: ScriptType,
        actual: ScriptType,
    },
    MultiFuzz(Vec<String>),
    UnknownCategory(String),
    Format(String),
    ScriptError(String),
    Empty,
}
impl std::fmt::Display for Error {
    // TODO: 好好實作它！
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<T: 'static + Send + Sync + std::error::Error> From<T> for Error {
    fn from(t: T) -> Self {
        Error::Others(vec![], Some(Box::new(t)))
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

pub type Result<T> = std::result::Result<T, Error>;

pub trait Contextabl<T> {
    fn context<S: ToString>(self, s: S) -> Result<T>;
}
impl<T> Contextabl<T> for Result<T> {
    fn context<S: ToString>(self, s: S) -> Result<T> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(e.context(s)),
        }
    }
}

impl<T, E: 'static + Send + Sync + std::error::Error> Contextabl<T> for std::result::Result<T, E> {
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
