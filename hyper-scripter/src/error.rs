use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct ExitCode(i32);
pub const EXIT_OK: ExitCode = ExitCode(0);
pub const EXIT_KNOWN_ERR: ExitCode = ExitCode(1);
pub const EXIT_OTHER_ERR: ExitCode = ExitCode(2);
impl ExitCode {
    /// 將另一個 `ExitCode` 和自身比較，若對方較嚴重，則將自身的值變成對方
    ///
    /// ```
    /// use hyper_scripter::error::*;
    /// let mut code = EXIT_KNOWN_ERR;
    /// code.cmp_and_replace(EXIT_OK);
    /// assert_eq!(code, EXIT_KNOWN_ERR);
    /// code.cmp_and_replace(EXIT_OTHER_ERR);
    /// assert_eq!(code, EXIT_OTHER_ERR);
    /// ```
    pub fn cmp_and_replace(&mut self, code: ExitCode) {
        self.0 = std::cmp::max(self.0, code.0);
    }
    pub fn code(&self) -> i32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SysPath {
    Config,
    Home,
}
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FormatCode {
    PromptLevel,
    Config,
    ScriptName,
    ScriptType,
    Regex,
    RangeQuery,
    ScriptQuery,
    Tag,
    NonEmptyArray,
}

#[derive(Debug, Clone)]
pub enum Error {
    Others(
        Vec<String>,
        Option<Arc<dyn 'static + Send + Sync + std::error::Error>>,
    ),
    SysPathNotFound(SysPath),
    EmptyCreate,

    PermissionDenied(Vec<PathBuf>),
    // NOTE: PathNotFound 比 ScriptNotFound 更嚴重，代表歷史記錄中有這支腳本，實際要找卻找不到
    PathNotFound(Vec<PathBuf>),
    GeneralFS(Vec<PathBuf>, Arc<std::io::Error>),

    PathExist(PathBuf),
    ScriptExist(String),
    ScriptIsFiltered(String),
    ScriptNotFound(String),
    NoAlias(String),
    UnknownType(String),
    Format(FormatCode, String),

    ScriptError(i32),
    PreRunError(i32),
    EditorError(i32, Vec<String>),

    RedundantOpt(RedundantOpt),
    TagSelectorNotFound(String),
    DontFuzz,
    NoPreviousArgs,
    Empty,

    Completion,
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
        if let Error::Others(msg, ..) = &mut self {
            msg.push(s.to_string());
        }
        self
    }
    pub fn code(&self) -> ExitCode {
        use Error::*;
        match self {
            Others(..) | GeneralFS(..) => EXIT_OTHER_ERR,
            ScriptError(c) | PreRunError(c) | EditorError(c, _) => ExitCode(*c),
            _ => EXIT_KNOWN_ERR,
        }
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
#[derive(Debug, Clone)]
pub enum RedundantOpt {
    Scripts(Vec<String>),
    Type,
    Tag,
    Selector,
}

impl From<RedundantOpt> for Error {
    fn from(opt: RedundantOpt) -> Self {
        Error::RedundantOpt(opt)
    }
}

// TODO: 一旦 specialization 穩了就直接把 StdError 實作在我們的錯誤結構上
#[derive(Display, Debug)]
pub struct DisplayError(Error);
impl From<Error> for DisplayError {
    fn from(err: Error) -> Self {
        DisplayError(err)
    }
}
impl DisplayError {
    pub fn into_err(self) -> Error {
        self.0
    }
}
impl std::error::Error for DisplayError {}
pub type DisplayResult<T = ()> = std::result::Result<T, DisplayError>;
