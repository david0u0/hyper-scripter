mod list_impl;
pub use list_impl::*;

mod grid;
pub use grid::Grid;

mod table_lib;
mod time_fmt;
mod tree;
mod tree_lib;

use crate::{
    error::{DisplayError, DisplayResult, Result},
    query::ListQuery,
    script::ScriptInfo,
};
use colored::{Color, ColoredString, Colorize};
use serde::Serialize;
use std::borrow::Cow;
use std::fmt::Write;
use std::num::NonZeroUsize;
use std::str::FromStr;

#[derive(Clone, Copy, Debug)]
struct LatestTxt(&'static str, &'static str);
const SHORT_LATEST_TXT: LatestTxt = LatestTxt("*", "");
const LONG_LATEST_TXT: LatestTxt = LatestTxt(" *", "  ");

macro_rules! style_name_w {
    ($w:expr, $plain:expr, $is_latest:expr, $latest_txt:expr, $color:expr, $name:expr) => {{
        use colored::{Color, Colorize};
        let mut width = $name.len();
        if $is_latest && !$plain {
            write!($w, "{}", $latest_txt.0.color(Color::Yellow).bold())?;
            width += $latest_txt.0.len();
        } else {
            write!($w, "{}", $latest_txt.1)?;
            width += $latest_txt.1.len();
        }
        let name = style($plain, $name, |s| {
            let s = s.color($color).bold();
            if $is_latest {
                s.underline()
            } else {
                s
            }
        });
        write!($w, "{}", name)?;
        width
    }};
}
pub(crate) use style_name_w;

fn style_name(
    plain: bool,
    is_latest: bool,
    latest_txt: LatestTxt,
    color: Color,
    name: &str,
) -> Result<(String, usize)> {
    let mut s = String::new();
    let width = style_name_w!(&mut s, plain, is_latest, latest_txt, color, name);
    Ok((s, width))
}

fn extract_help(script: &ScriptInfo) -> String {
    let mut buff = String::new();
    fn inner(buff: &mut String, script: &ScriptInfo) -> Result {
        let script_path = crate::path::open_script(&script.name, &script.ty, Some(true))?;
        *buff = crate::util::read_file(&script_path)?;
        Ok(())
    }
    match inner(&mut buff, script) {
        Err(e) => {
            log::warn!("讀取腳本失敗{}，直接回空的幫助字串", e);
            return String::new();
        }
        Ok(()) => (),
    };
    let mut helps = crate::extract_msg::extract_help_from_content(&buff);
    helps.next().unwrap_or_default().to_owned()
}

fn exec_time_str(script: &ScriptInfo) -> Cow<'static, str> {
    match &script.exec_time {
        None => Cow::Borrowed("Never"),
        Some(t) => Cow::Owned(format!("{} ({})", time_fmt::fmt(t), script.exec_count)),
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
fn style<T: AsRef<str>, F: for<'a> FnOnce(&'a str) -> ColoredString>(
    plain: bool,
    s: T,
    f: F,
) -> ColoredString {
    if plain {
        s.as_ref().normal()
    } else {
        f(s.as_ref())
    }
}

pub fn get_screen_width() -> u16 {
    console::Term::stdout().size_checked().map_or(0, |s| s.1)
}
