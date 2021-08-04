mod list_impl;
pub use list_impl::*;

mod tree;
mod tree_lib;

use crate::{error::Result, query::ListQuery, script::ScriptInfo, script_time::ScriptTime};
use colored::{ColoredString, Colorize};
use std::borrow::Cow;

fn extract_help<'a>(buff: &'a mut String, script: &ScriptInfo) -> Result<&'a str> {
    let script_path = crate::path::open_script(&script.name, &script.ty, None)?;
    *buff = crate::util::read_file(&script_path)?;
    let mut helps = crate::extract_msg::extract_help_from_content(buff);
    Ok(helps.next().unwrap_or_default())
}

fn time_str<T>(time: &Option<ScriptTime<T>>) -> Cow<'static, str> {
    match time {
        None => Cow::Borrowed("Never"),
        Some(t) => Cow::Owned(t.to_string()),
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DisplayIdentStyle {
    File,
    Name,
    Normal,
    NameAndFile,
}
#[derive(Debug, Eq, PartialEq)]
pub enum DisplayStyle<T, U> {
    Short(DisplayIdentStyle, U),
    Long(T),
}
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Grouping {
    Tag,
    Tree,
    None,
}
impl Grouping {
    pub fn is_none(self) -> bool {
        self == Grouping::None
    }
}
impl<T: AsRef<str>> From<T> for Grouping {
    fn from(s: T) -> Self {
        match s.as_ref() {
            "tag" => Grouping::Tag,
            "tree" => Grouping::Tree,
            "none" => Grouping::None,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub struct ListOptions<'a, T = (), U = ()> {
    pub grouping: Grouping,
    pub queries: &'a [ListQuery],
    pub plain: bool,
    pub display_style: DisplayStyle<T, U>,
}

#[inline]
fn style<T: AsRef<str>, F: FnOnce(ColoredString) -> ColoredString>(
    plain: bool,
    s: T,
    f: F,
) -> ColoredString {
    let s = s.as_ref().normal();
    if !plain {
        f(s)
    } else {
        s
    }
}
