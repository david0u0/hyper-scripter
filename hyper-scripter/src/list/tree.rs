use super::{
    style, time_str,
    tree_lib::{self, TreeFormatter},
    DisplayIdentStyle, DisplayStyle, ListOptions,
};
use crate::config::Config;
use crate::error::Result;
use crate::extract_help;
use crate::script::ScriptInfo;
use colored::{Color, Colorize};
use fxhash::FxHashMap as HashMap;
use prettytable::{cell, format, row, Row, Table};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::io::Write;

const TITLE: &[&str] = &[
    "category",
    "last write time",
    "last execute time",
    "help message",
];

struct ShortFormatter {
    plain: bool,
    ident_style: DisplayIdentStyle,
    latest_script_id: i64,
}
struct LongFormatter {
    table: Table,
    plain: bool,
    latest_script_id: i64,
}
struct TrimmedScriptInfo<'b>(Cow<'b, str>, &'b ScriptInfo);

fn ident_string<'b>(style: DisplayIdentStyle, t: &TrimmedScriptInfo<'b>) -> Result<String> {
    let TrimmedScriptInfo(name, script) = t;
    Ok(match style {
        DisplayIdentStyle::Normal => format!("{}({})", name, script.ty),
        DisplayIdentStyle::File => script.file_path()?.to_string_lossy().to_string(),
        DisplayIdentStyle::Name => name.to_string(),
    })
}

impl<'b> tree_lib::TreeValue<'b> for TrimmedScriptInfo<'b> {
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
impl<'b, W: Write> TreeFormatter<'b, TrimmedScriptInfo<'b>, W> for ShortFormatter {
    fn fmt_leaf(&mut self, f: &mut W, t: &TrimmedScriptInfo<'b>) -> Result {
        let TrimmedScriptInfo(_, script) = t;
        let color = Config::get()?.get_color(&script.ty)?;
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
impl<'b, W: Write> TreeFormatter<'b, TrimmedScriptInfo<'b>, W> for LongFormatter {
    fn fmt_leaf(&mut self, f: &mut W, t: &TrimmedScriptInfo<'b>) -> Result {
        let TrimmedScriptInfo(name, script) = t;
        let color = Config::get()?.get_color(&script.ty)?;
        let ident = style(self.plain, name, |s| s.color(color).bold());
        let ty_txt = style(self.plain, &script.ty, |s| s.color(color).bold());
        if self.latest_script_id == script.id && !self.plain {
            write!(f, "{}", "*".color(Color::Yellow).bold())?;
        }
        write!(f, "{}", ident)?;

        extract_help!(help_msg, script, false);
        let help_msg = help_msg.into_iter().next().unwrap_or_default();

        let row = row![c->ty_txt, c->script.write_time, c->time_str(&script.exec_time), help_msg];
        self.table.add_row(row);
        Ok(())
    }
    fn fmt_nonleaf(&mut self, f: &mut W, t: &str) -> Result {
        let ident = style(self.plain, t, |s| s.dimmed().italic());
        let empty = style(self.plain, "----", |s| s.dimmed());
        write!(f, "{}", ident)?;
        self.table
            .add_row(Row::new(vec![cell!(c->empty).with_hspan(TITLE.len())]));
        Ok(())
    }
}

type TreeNode<'b> = tree_lib::TreeNode<'b, TrimmedScriptInfo<'b>>;

fn build_forest(scripts: Vec<&ScriptInfo>) -> Vec<TreeNode<'_>> {
    let mut m = HashMap::default();
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
    let mut forest: Vec<_> = m.into_iter().map(|(_, t)| t).collect();
    forest.sort_by(|a, b| a.cmp(b));
    forest
}

pub fn fmt<W: Write>(
    scripts: Vec<&ScriptInfo>,
    latest_script_id: i64,
    opt: &mut ListOptions<Table, &mut W>,
) -> Result<()> {
    let forest = build_forest(scripts);
    match &mut opt.display_style {
        DisplayStyle::Long(table) => {
            let mut right_table = Table::new();
            right_table.set_format(*format::consts::FORMAT_CLEAN);
            right_table.set_titles(Row::new(TITLE.iter().map(|t| cell!(c->t)).collect()));
            let mut fmter = LongFormatter {
                plain: opt.plain,
                latest_script_id,
                table: right_table,
            };
            let mut left = Vec::<u8>::new();
            writeln!(left, "")?;
            fmter.fmt_all(&mut left, forest.into_iter())?;
            let left = std::str::from_utf8(&left)?;
            table.add_row(row![left, fmter.table.to_string()]);
        }
        DisplayStyle::Short(ident_style, w) => {
            let mut fmter = ShortFormatter {
                plain: opt.plain,
                ident_style: *ident_style,
                latest_script_id,
            };
            fmter.fmt_all(w, forest.into_iter())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::script::IntoScriptName;
    use chrono::NaiveDateTime;

    fn build(v: Vec<(&'static str, &'static str)>) -> Vec<ScriptInfo> {
        v.into_iter()
            .enumerate()
            .map(|(id, (name, ty))| {
                let id = id as i64;
                let time = NaiveDateTime::from_timestamp(id, 0);
                ScriptInfo::builder(
                    id,
                    name.into_script_name().unwrap(),
                    ty.into(),
                    vec![].into_iter(),
                )
                .created_time(time)
                .build()
            })
            .collect()
    }
    #[test]
    fn test_fmt_tree_short() {
        let _ = env_logger::try_init();
        let scripts = build(vec![
            ("bbb/ccc/ggg/rrr", "tmux"),
            ("aaa/bbb", "rb"),
            ("bbb/ccc/ddd", "tmux"),
            ("bbb/ccc/ggg/fff", "tmux"),
            ("aaa", "sh"),
            ("bbb/ccc/ddd/eee", "tmux"),
            (".2", "txt"),
            ("bbb/ccc/yyy", "js"),
            ("bbb/ccc/ddd/www", "rb"),
            ("bbb/ccc/ggg/xxx", "tmux"),
            ("bbb/ddd", "tmux"),
        ]);
        let forest = build_forest(scripts.iter().collect());
        let mut fmter = ShortFormatter {
            plain: true,
            ident_style: DisplayIdentStyle::Normal,
            latest_script_id: 1,
        };
        let ans = "
.2(txt)
aaa(sh)
aaa
└── bbb(rb)
bbb
├── ddd(tmux)
└── ccc
    ├── yyy(js)
    ├── ddd(tmux)
    ├── ddd
    │   ├── www(rb)
    │   └── eee(tmux)
    └── ggg
        ├── xxx(tmux)
        ├── fff(tmux)
        └── rrr(tmux)
"
        .trim();
        let mut v8 = Vec::<u8>::new();
        fmter.fmt_all(&mut v8, forest.into_iter()).unwrap();
        assert_eq!(std::str::from_utf8(&v8).unwrap().trim(), ans);
    }
}
