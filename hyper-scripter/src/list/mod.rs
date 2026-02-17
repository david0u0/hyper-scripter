mod list_impl;
pub use list_impl::*;

mod grid;
pub use grid::Grid;

mod table_lib;
mod time_fmt;
mod tree;
mod tree_lib;

use crate::color::{Color, StyleObj, Stylize};
use crate::util::writable::{write_writable, FmtWrite, Writable};
use crate::{
    error::{DisplayError, DisplayResult, FormatCode::Template as TemplateCode, Result},
    script::ScriptInfo,
};
use clap::ValueEnum;
use handlebars::Template;
use serde::Serialize;
use std::borrow::Cow;
use std::num::NonZeroUsize;
use std::str::FromStr;

#[derive(Clone, Copy, Debug)]
struct LatestTxt(&'static str, &'static str);
const SHORT_LATEST_TXT: LatestTxt = LatestTxt("*", "");
const LONG_LATEST_TXT: LatestTxt = LatestTxt(" *", "  ");

fn style_name_w(
    mut w: impl Writable,
    plain: bool,
    is_latest: bool,
    latest_txt: LatestTxt,
    color: Color,
    name: &str,
) -> Result<usize> {
    let mut width = name.len();
    if is_latest && !plain {
        write_writable!(w, "{}", latest_txt.0.stylize().color(Color::Yellow).bold())?;
        width += latest_txt.0.len();
    } else {
        write_writable!(w, "{}", latest_txt.1)?;
        width += latest_txt.1.len();
    }
    let name = style(plain, name, |s| {
        s.color(color).bold();
        if is_latest {
            s.underline();
        }
    });
    write_writable!(w, "{}", name)?;
    Ok(width)
}

fn style_name(
    plain: bool,
    is_latest: bool,
    latest_txt: LatestTxt,
    color: Color,
    name: &str,
) -> Result<(String, usize)> {
    let mut s = String::new();
    let width = style_name_w(FmtWrite(&mut s), plain, is_latest, latest_txt, color, name)?;
    Ok((s, width))
}

fn extract_help(script: &ScriptInfo) -> String {
    fn inner(script: &ScriptInfo) -> Result<impl Iterator<Item = String>> {
        let script_path = crate::path::open_script(&script.name, &script.ty, Some(true))?;
        let content = crate::util::read_file_lines(&script_path)?;
        Ok(content)
    }
    let content = match inner(script) {
        Err(e) => {
            log::warn!("讀取腳本失敗{}，直接回空的幫助字串", e);
            return String::new();
        }
        Ok(c) => c,
    };
    let mut helps = crate::extract_msg::extract_help_from_content(content);
    helps.next().map(|x| x.to_string()).unwrap_or_default()
}

fn exec_time_str(script: &ScriptInfo) -> Cow<'static, str> {
    match &script.exec_time {
        None => Cow::Borrowed("Never"),
        Some(t) => Cow::Owned(format!("{} ({})", time_fmt::fmt(t), script.exec_count)),
    }
}

#[derive(Debug)]
pub enum DisplayStyle<T, U> {
    Short(IdentTemplate, U),
    Long(T),
}
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, ValueEnum)]
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

#[derive(Debug)]
pub struct ListOptions<T = (), U = ()> {
    pub grouping: Grouping,
    pub plain: bool,
    pub limit: Option<NonZeroUsize>,
    pub display_style: DisplayStyle<T, U>,
}

#[inline]
fn style<T: std::fmt::Display, F: for<'a> FnOnce(&'a mut StyleObj<T>)>(
    plain: bool,
    s: T,
    f: F,
) -> StyleObj<T> {
    let mut s = s.stylize();
    if !plain {
        f(&mut s);
    }
    s
}

pub fn get_screen_width() -> u16 {
    console::Term::stdout().size_checked().map_or(0, |s| s.1)
}

#[derive(Debug)]
pub enum IdentTemplate {
    Classic,
    Name,
    File,
    ID,
    General(Template),
}
impl FromStr for IdentTemplate {
    type Err = DisplayError;
    fn from_str(s: &str) -> DisplayResult<Self> {
        let t = match s {
            "{{name}}({{ty}})" => IdentTemplate::Classic,
            "{{file}}" => IdentTemplate::File,
            "{{name}}" => IdentTemplate::Name,
            "{{id}}" => IdentTemplate::ID,
            _ => match Template::compile(s) {
                Err(_) => return TemplateCode.to_display_res(s.to_owned()),
                Ok(t) => IdentTemplate::General(t),
            },
        };
        Ok(t)
    }
}
