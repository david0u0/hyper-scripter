#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate derive_more;
pub mod args;
pub mod color;
pub mod config;
pub mod db;
pub mod env_pair;
pub mod error;
mod error_display;
pub mod extract_msg;
pub mod fuzzy;
pub mod list;
pub mod migration;
pub mod my_env_logger;
pub mod path;
pub mod process_lock;
pub mod query;
pub mod script;
pub mod script_repo;
pub mod script_time;
pub mod script_type;
pub mod state;
pub mod tag;
pub mod util;

pub use std::borrow::Cow;

pub const APP_NAME: &str = "hs";
pub const SEP: &str = "/";
#[derive(Debug)]
pub enum Either<T, U> {
    One(T),
    Two(U),
}

/// copied from `shell_escape` crate
pub fn to_display_args(arg: &str) -> Cow<'_, str> {
    fn non_whitelisted(ch: char) -> bool {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '=' | '/' | ',' | '.' | '+' => false,
            _ => true,
        }
    }
    if !arg.is_empty() && !arg.contains(non_whitelisted) {
        return Cow::Borrowed(arg);
    }

    let mut es = String::with_capacity(arg.len() + 2);
    es.push('\'');
    for ch in arg.chars() {
        match ch {
            '\'' | '!' => {
                es.push_str("'\\");
                es.push(ch);
                es.push('\'');
            }
            _ => es.push(ch),
        }
    }
    es.push('\'');
    Cow::Owned(es)
}

#[cfg(not(any(feature = "no-log", feature = "log",)))]
compile_error!("one of the features [log/no-log] must be enabled");

#[cfg(all(feature = "no-log", feature = "log",))]
compile_error!("only one of the features [log/no-log] can be enabled");
