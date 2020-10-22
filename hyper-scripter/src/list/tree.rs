use super::{
    get_color, style,
    tree_lib::{self, TreeFormatter},
    DisplayIdentStyle, DisplayStyle, ListOptions,
};
use crate::error::Result;
use crate::script::{ScriptInfo, ScriptName};
use chrono::NaiveDateTime;
use colored::{Color, Colorize};
use prettytable::{cell, format, row, Cell, Row, Table};
use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::Hash;
use std::io::Write;

/*
struct ShortFormatter {
    plain: bool,
    ident_style: DisplayIdentStyle,
    latest_script_id: i64,
}
impl<'a, 'b> tree_lib::TreeValue for &'b ScriptInfo<'a> {
    type Key = NaiveDateTime;
    fn sort_key(&self) -> Self::Key {
        self.last_time()
    }
    fn display_key(&self) -> Cow<str> {
        self.name.key()
    }
}
impl<'a, 'b, W: Write> TreeFormatter<'a, &'b ScriptInfo<'a>, W> for ShortFormatter {
    fn fmt_leaf(&mut self, f: &mut W, t: &&'b ScriptInfo<'a>) -> Result {
        let ident = self.ident_style.ident_string(t)?;
        let color = get_color(t)?;
        let ident = style(self.plain, ident, |s| s.color(color).bold());
        write!(f, "{}", ident)?;
        Ok(())
    }
    fn fmt_nonleaf(&mut self, f: &mut W, t: &str) -> Result {
        write!(f, "{}", t)?;
        Ok(())
    }
}

type TreeNode<'a, 'b> = tree_lib::TreeNode<'a, &'b ScriptInfo<'a>>;

fn build_tree<'a, 'b>(scripts: Vec<&'b ScriptInfo<'a>>) -> Vec<TreeNode<'a, 'b>> {
    vec![]
}

pub fn fmt<W: Write>(
    scripts: Vec<&ScriptInfo>,
    latest_script_id: i64,
    opt: &mut ListOptions<Table, &mut W>,
) -> Result<()> {
    match &mut opt.display_style {
        DisplayStyle::Long(table) => {
            panic!();
        }
        DisplayStyle::Short(ident_style, w) => {
            let mut fmter = ShortFormatter {
                plain: opt.plain,
                ident_style: *ident_style,
                latest_script_id,
            };
            let mut node = tree_lib::TreeNode::new_leaf(scripts[0]);
            fmter.fmt(w, &mut node)?;
        }
    }
    Ok(())
}

*/
