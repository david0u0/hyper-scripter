#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate derive_more;
pub mod args;
pub mod config;
pub mod db;
pub mod error;
mod error_display;
pub mod extract_help;
pub mod fuzzy;
pub mod list;
pub mod migration;
pub mod path;
pub mod query;
pub mod script;
pub mod script_repo;
pub mod script_time;
pub mod script_type;
pub mod tag;
pub mod util;

pub enum Either<T, U> {
    One(T),
    Two(U),
}

#[macro_export]
macro_rules! extract_help {
    ($res_name:ident, $script:expr, $long:expr) => {
        let script_path = $crate::path::open_script(&$script.name, &$script.ty, Some(true))?;
        let content = $crate::util::read_file(&script_path)?;
        let $res_name = $crate::extract_help::extract_help_from_content(&content, $long);
    };
}
