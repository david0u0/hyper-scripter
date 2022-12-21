mod list_impl;
pub use list_impl::*;

mod grid;
pub use grid::Grid;

mod tree;
mod tree_lib;

use crate::{
    error::{DisplayError, DisplayResult, Result},
    query::ListQuery,
    script::ScriptInfo,
    script_time::ScriptTime,
    state::State,
};
use chrono::{Datelike, Local, TimeZone, Utc};
use colored::{ColoredString, Colorize};
use serde::Serialize;
use std::borrow::Cow;
use std::num::NonZeroUsize;
use std::str::FromStr;

static THIS_YEAR: State<i32> = State::new();

fn init_this_year() {
    let now = Utc::now().naive_local();
    THIS_YEAR.set(now.year());
}

// TODO: return something impl Display?
fn fmt_time<T>(time: &ScriptTime<T>) -> String {
    let local_time = Local.from_utc_datetime(&**time);
    if time.year() == *THIS_YEAR.get() {
        format!("{}", local_time.format("%m-%d %H:%M"))
    } else {
        format!("{}", local_time.format("%m-%d %Y"))
    }
}

fn extract_help<'a>(buff: &'a mut String, script: &ScriptInfo) -> &'a str {
    fn inner(buff: &mut String, script: &ScriptInfo) -> Result {
        let script_path = crate::path::open_script(&script.name, &script.ty, Some(true))?;
        *buff = crate::util::read_file(&script_path)?;
        Ok(())
    }
    match inner(buff, script) {
        Err(e) => {
            log::warn!("讀取腳本失敗{}，直接回空的幫助字串", e);
            return "";
        }
        Ok(p) => p,
    };
    let mut helps = crate::extract_msg::extract_help_from_content(buff);
    helps.next().unwrap_or_default()
}

fn exec_time_str(script: &ScriptInfo) -> Cow<'static, str> {
    match &script.exec_time {
        None => Cow::Borrowed("Never"),
        Some(t) => Cow::Owned(format!("{}({})", fmt_time(t), script.exec_count)),
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
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
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
impl Default for Grouping {
    fn default() -> Self {
        Grouping::None
    }
}
impl FromStr for Grouping {
    type Err = DisplayError;
    fn from_str(s: &str) -> DisplayResult<Self> {
        let g = match s {
            "tag" => Grouping::Tag,
            "tree" => Grouping::Tree,
            "none" => Grouping::None,
            _ => unreachable!(),
        };
        Ok(g)
    }
}

#[derive(Debug)]
pub struct ListOptions<T = (), U = ()> {
    pub grouping: Grouping,
    pub plain: bool,
    pub limit: Option<NonZeroUsize>,
    pub queries: Vec<ListQuery>,
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
