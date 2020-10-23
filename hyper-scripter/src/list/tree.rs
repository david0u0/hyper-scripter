use super::{
    get_color, style,
    tree_lib::{self, TreeFormatter},
    DisplayIdentStyle, DisplayStyle, ListOptions,
};
use crate::error::Result;
use crate::script::ScriptInfo;
use colored::{Color, Colorize};
use prettytable::{cell, row, Cell, Row, Table};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::io::Write;

struct ShortFormatter {
    plain: bool,
    ident_style: DisplayIdentStyle,
    latest_script_id: i64,
}
struct TrimmedScriptInfo<'b, 'a: 'b>(Cow<'b, str>, &'b ScriptInfo<'a>);

fn ident_string<'b, 'a>(style: DisplayIdentStyle, t: &TrimmedScriptInfo<'b, 'a>) -> Result<String> {
    let TrimmedScriptInfo(name, script) = t;
    Ok(match style {
        DisplayIdentStyle::Normal => format!("{}({})", name, script.ty),
        DisplayIdentStyle::File => script.file_path()?.to_string_lossy().to_string(),
        DisplayIdentStyle::Name => name.to_string(),
    })
}

impl<'b, 'a: 'b> tree_lib::TreeValue<'b> for TrimmedScriptInfo<'b, 'a> {
    fn tree_cmp(&self, other: &Self) -> Ordering {
        other.1.last_time().cmp(&self.1.last_time())
    }
    fn display_key(&self) -> Cow<'b, str> {
        match &self.0 {
            Cow::Borrowed(s) => Cow::Borrowed(s),
            Cow::Owned(_) => self.1.name.key(),
        }
    }
}
impl<'b, 'a: 'b, W: Write> TreeFormatter<'b, TrimmedScriptInfo<'b, 'a>, W> for ShortFormatter {
    fn fmt_leaf(&mut self, f: &mut W, t: &TrimmedScriptInfo<'b, 'a>) -> Result {
        let TrimmedScriptInfo(_, script) = t;
        let color = get_color(script)?;
        let ident = ident_string(self.ident_style, t)?;
        let ident = style(self.plain, ident, |s| s.color(color).bold());
        if self.latest_script_id == script.id && !self.plain {
            write!(f, "{}", "*".color(Color::Yellow).bold())?;
        }
        write!(f, "{}", ident)?;
        Ok(())
    }
    fn fmt_nonleaf(&mut self, f: &mut W, t: &str) -> Result {
        let ident = style(self.plain, t, |s| s.dimmed().italic());
        write!(f, "{}", ident)?;
        Ok(())
    }
}

type TreeNode<'a, 'b> = tree_lib::TreeNode<'b, TrimmedScriptInfo<'b, 'a>>;

fn build_tree<'a, 'b>(
    scripts: Vec<&'b ScriptInfo<'a>>,
) -> tree_lib::Childs<'b, TrimmedScriptInfo<'b, 'a>> {
    let mut m = HashMap::new();
    for script in scripts.into_iter() {
        let name = script.name.key();
        let name_key = match name {
            Cow::Borrowed(s) => s,
            _ => {
                m.insert(
                    (false, name.clone()),
                    TreeNode::new_leaf(TrimmedScriptInfo(name, script)),
                );
                continue;
            }
        };
        let mut path: Vec<_> = name_key.split("/").collect();
        let name = Cow::Borrowed(path.pop().unwrap());
        let leaf = TreeNode::new_leaf(TrimmedScriptInfo(name, script));
        TreeNode::insert_to_map(&mut m, &path, leaf);
    }
    m
}

pub fn fmt<W: Write>(
    scripts: Vec<&ScriptInfo>,
    latest_script_id: i64,
    opt: &mut ListOptions<Table, &mut W>,
) -> Result<()> {
    let tree = build_tree(scripts);
    log::debug!("樹狀圖：{:?}", tree);
    let mut tree: Vec<_> = tree.into_iter().map(|(_, n)| n).collect();
    tree.sort_by(|a, b| a.cmp(b));
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
            for mut root in tree.into_iter() {
                fmter.fmt(w, &mut root)?;
                writeln!(w, "")?;
            }
        }
    }
    Ok(())
}
