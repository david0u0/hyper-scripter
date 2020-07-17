use std::path::PathBuf;
#[derive(Debug)]
pub enum Others {}
#[derive(Debug)]
pub enum Error {
    Others(
        Vec<String>,
        Option<Box<dyn 'static + Send + Sync + std::error::Error>>,
    ),
    PathNotFound(PathBuf),
    PathNotSet,
    NoSuchScript(PathBuf),
    Format(String),
    EmptyAnonymous,
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

pub trait Contextabl {
    fn context<T: ToString>(self, s: T) -> Self;
}
impl<T> Contextabl for Result<T> {
    fn context<S: ToString>(self, s: S) -> Self {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(e.context(s)),
        }
    }
}
