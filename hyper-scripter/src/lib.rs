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
pub mod path;
pub mod query;
pub mod script;
pub mod script_repo;
pub mod script_time;
pub mod script_type;
pub mod state;
pub mod tag;
pub mod util;

pub const APP_NAME: &str = "hs";
pub const SEP: &str = "/";
pub enum Either<T, U> {
    One(T),
    Two(U),
}
