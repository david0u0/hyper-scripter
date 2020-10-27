mod list;
pub use list::*;

mod tree;
mod tree_lib;

use crate::{
    config::Config, error::Result, query::ListQuery, script::ScriptInfo, script_time::ScriptTime,
};
use colored::{Color, ColoredString, Colorize};
use std::borrow::Cow;

fn time_str<T: Clone + std::fmt::Debug>(time: &Option<ScriptTime<T>>) -> Cow<'static, str> {
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
fn get_color(script: &ScriptInfo) -> Result<Color> {
    let c = Config::get()?.get_script_conf(&script.ty)?.color.as_str();
    Ok(Color::from(c))
}