use super::{
    exec_time_str, extract_help, style, style_name_w,
    table_lib::{Cell, Table},
    time_fmt,
    tree_lib::{self, LeadingDisplay, TreeFormatter},
    DisplayIdentStyle, DisplayStyle, ListOptions, SHORT_LATEST_TXT,
};
use crate::error::Result;
use crate::script::ScriptInfo;
use crate::util::get_display_type;
use colored::Colorize;
use fxhash::FxHashMap as HashMap;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt::Write as FmtWrite;
use std::io::Write;
use unicode_width::UnicodeWidthStr;

struct ShortFormatter<W: Write> {
    w: W,
    plain: bool,
    ident_style: DisplayIdentStyle,
    latest_script_id: i64,
}
struct LongFormatter<'a> {
    table: &'a mut Table,
    plain: bool,
    latest_script_id: i64,
}
struct TrimmedScriptInfo<'b>(Cow<'b, str>, &'b ScriptInfo);

fn ident_string(style: DisplayIdentStyle, ty: &str, t: &TrimmedScriptInfo<'_>) -> String {
    let TrimmedScriptInfo(name, script) = t;
    match style {
        DisplayIdentStyle::Normal => format!("{}({})", name, ty),
        DisplayIdentStyle::File => script.file_path_fallback().to_string_lossy().to_string(),
        DisplayIdentStyle::Name => name.to_string(),
        DisplayIdentStyle::NameAndFile => format!(
            "{}({})",
            name.to_string(),
            script.file_path_fallback().to_string_lossy().to_string()
        ),
    }
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
impl<'b, W: Write> TreeFormatter<'b, TrimmedScriptInfo<'b>> for ShortFormatter<W> {
    fn fmt_leaf(&mut self, l: LeadingDisplay, t: &TrimmedScriptInfo<'b>) -> Result {
        let TrimmedScriptInfo(_, script) = t;
        let ty = get_display_type(&script.ty);
        let ident = ident_string(self.ident_style, &*ty.display(), t);
        write!(self.w, "{}", l)?;
        style_name_w!(
            self.w,
            self.plain,
            self.latest_script_id == script.id,
            SHORT_LATEST_TXT,
            ty.color(),
            &ident
        );
        writeln!(self.w, "")?;
        Ok(())
    }
    fn fmt_nonleaf(&mut self, l: LeadingDisplay, t: &str) -> Result {
        let ident = style(self.plain, t, |s| s.dimmed().italic());
        writeln!(self.w, "{}{}", l, ident)?;
        Ok(())
    }
}

impl<'b> TreeFormatter<'b, TrimmedScriptInfo<'b>> for LongFormatter<'b> {
    fn fmt_leaf(&mut self, l: LeadingDisplay, t: &TrimmedScriptInfo<'b>) -> Result {
        let TrimmedScriptInfo(name, script) = t;
        let ty = get_display_type(&script.ty);
        let color = ty.color();

        let mut ident_txt = l.to_string();
        let mut ident_width = ident_txt.width();
        {
            let name_width = style_name_w!(
                &mut ident_txt,
                self.plain,
                self.latest_script_id == script.id,
                SHORT_LATEST_TXT,
                ty.color(),
                &name
            );
            ident_width += name_width;
        }

        let ty = ty.display();
        let ty_width = ty.len();
        let ty_txt = style(self.plain, ty, |s| s.color(color).bold());

        let help_msg = extract_help(script);

        let row = vec![
            Cell::new_with_len(ident_txt, ident_width),
            Cell::new_with_len(ty_txt.to_string(), ty_width),
            Cell::new(time_fmt::fmt(&script.write_time).to_string()),
            Cell::new(exec_time_str(script).to_string()),
            Cell::new(help_msg),
        ];
        self.table.add_row(row);
        Ok(())
    }
    fn fmt_nonleaf(&mut self, l: LeadingDisplay, name: &str) -> Result {
        let mut ident_txt = l.to_string();
        let mut ident_width = ident_txt.width();
        let name = style(self.plain, name, |s| s.dimmed().italic());
        ident_width += name.len();
        write!(&mut ident_txt, "{}", name)?;
        let row = vec![Cell::new_with_len(ident_txt, ident_width)];
        self.table.add_row(row);
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
        let mut path: Vec<_> = name_key.split('/').collect();
        let name = Cow::Borrowed(path.pop().unwrap());
        let leaf = TreeNode::new_leaf(TrimmedScriptInfo(name, script));
        TreeNode::insert_to_map(&mut m, &path, leaf);
    }
    let mut forest: Vec<_> = m.into_iter().map(|(_, t)| t).collect();
    forest.sort_by(|a, b| a.simple_cmp(b));
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
            let mut fmter = LongFormatter {
                plain: opt.plain,
                latest_script_id,
                table,
            };
            fmter.fmt_all(forest.into_iter())?;
        }
        DisplayStyle::Short(ident_style, w) => {
            let mut fmter = ShortFormatter {
                w,
                plain: opt.plain,
                ident_style: *ident_style,
                latest_script_id,
            };
            fmter.fmt_all(forest.into_iter())?;
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
                let mut builder = ScriptInfo::builder(
                    id,
                    name.to_owned().into_script_name().unwrap(),
                    ty.into(),
                    vec![].into_iter(),
                );
                builder.created_time(time);
                builder.build()
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
            w: Vec::<u8>::new(),
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
        fmter.fmt_all(forest.into_iter()).unwrap();
        assert_eq!(std::str::from_utf8(&fmter.w).unwrap().trim(), ans);
    }
}
