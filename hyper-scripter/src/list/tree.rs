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
use fxhash::FxHashMap as HashMap;
use std::borrow::Cow;
use std::fmt::Write as FmtWrite;
use std::io::Write;

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
    type CmpKey = u64;
    fn cmp_key(&self) -> u64 {
        let s = &self.1;
        if s.exec_time.is_none() {
            0
        } else {
            s.exec_count
        }
    }
    fn display_key(&self) -> Cow<'b, str> {
        match &self.0 {
            Cow::Borrowed(s) => Cow::Borrowed(s),
            Cow::Owned(_) => self.1.name.key(),
        }
    }
}
impl<'b, W: Write> TreeFormatter<'b, TrimmedScriptInfo<'b>, u64> for ShortFormatter<W> {
    fn fmt_leaf(&mut self, l: LeadingDisplay, t: &TrimmedScriptInfo<'b>) -> Result {
        let TrimmedScriptInfo(_, script) = t;
        let ty = get_display_type(&script.ty);
        let ident = ident_string(self.ident_style, &*ty.display(), t);
        let l = style(self.plain, l, |s| s.dimmed().done());
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
        let ident = style(self.plain, t, |s| s.dimmed().italic().done());
        let l = style(self.plain, l, |s| s.dimmed().done());
        writeln!(self.w, "{}{}", l, ident)?;
        Ok(())
    }
}

impl<'b> TreeFormatter<'b, TrimmedScriptInfo<'b>, u64> for LongFormatter<'b> {
    fn fmt_leaf(&mut self, l: LeadingDisplay, t: &TrimmedScriptInfo<'b>) -> Result {
        let TrimmedScriptInfo(name, script) = t;
        let ty = get_display_type(&script.ty);
        let color = ty.color();

        let mut ident_width = l.width();
        let mut ident_txt = style(self.plain, l, |s| s.dimmed().done()).to_string();
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
        let ty_txt = style(self.plain, ty, |s| s.color(color).bold().done());

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
        let mut ident_width = l.width();
        let mut ident_txt = style(self.plain, l, |s| s.dimmed().done()).to_string();
        ident_width += name.len();
        let name = style(self.plain, name, |s| s.dimmed().italic().done());
        write!(&mut ident_txt, "{}", name)?;
        let row = vec![Cell::new_with_len(ident_txt, ident_width)];
        self.table.add_row(row);
        Ok(())
    }
}

type TreeNode<'b> = tree_lib::TreeNode<'b, TrimmedScriptInfo<'b>, u64>;

fn build_forest(scripts: Vec<&ScriptInfo>) -> TreeNode<'_> {
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
    TreeNode::new_nonleaf(".", m)
}

pub fn fmt<W: Write>(
    scripts: Vec<&ScriptInfo>,
    latest_script_id: i64,
    opt: &mut ListOptions<Table, &mut W>,
) -> Result<()> {
    let mut root = build_forest(scripts);
    match &mut opt.display_style {
        DisplayStyle::Long(table) => {
            let mut fmter = LongFormatter {
                plain: opt.plain,
                latest_script_id,
                table,
            };
            fmter.fmt(&mut root)?;
        }
        DisplayStyle::Short(ident_style, w) => {
            let mut fmter = ShortFormatter {
                w,
                plain: opt.plain,
                ident_style: *ident_style,
                latest_script_id,
            };
            fmter.fmt(&mut root)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{my_env_logger, script::IntoScriptName};
    use chrono::NaiveDateTime;

    fn build(v: Vec<(&'static str, &'static str)>) -> Vec<ScriptInfo> {
        v.into_iter()
            .enumerate()
            .map(|(idx, (name, ty))| {
                let idx = idx as i64;
                let time = NaiveDateTime::from_timestamp(idx, 0);
                let mut builder = ScriptInfo::builder(
                    idx,
                    name.to_owned().into_script_name().unwrap(),
                    ty.into(),
                    vec![].into_iter(),
                );
                builder.created_time(time);
                builder.exec_time(time);
                builder.exec_count(idx as u64);
                builder.build()
            })
            .collect()
    }
    #[test]
    fn test_fmt_tree_short() {
        let _ = my_env_logger::try_init();
        let scripts = build(vec![
            ("bbb/ccc/ggg/rrr", "tmux"),
            ("bbb/ccc/ddd", "tmux"),
            ("bbb/ccc/ggg/fff", "tmux"),
            ("aaa", "sh"),
            ("bbb/ccc/ddd/eee", "tmux"),
            (".2", "txt"),
            ("bbb/ccc/yyy", "js"),
            ("bbb/ccc/ddd/www", "rb"),
            ("bbb/ccc/ggg/xxx", "tmux"),
            ("bbb/ddd", "tmux"),
            ("aaa/bbb", "rb"),
        ]);
        let mut root = build_forest(scripts.iter().collect());
        let mut fmter = ShortFormatter {
            w: Vec::<u8>::new(),
            plain: true,
            ident_style: DisplayIdentStyle::Normal,
            latest_script_id: 1,
        };
        let ans = "
.
├── aaa(sh)
├── .2(txt)
├── bbb
│  ├── ddd(tmux)
│  └── ccc
│     ├── ddd(tmux)
│     ├── yyy(js)
│     ├── ddd
│     │  ├── eee(tmux)
│     │  └── www(rb)
│     └── ggg
│        ├── rrr(tmux)
│        ├── fff(tmux)
│        └── xxx(tmux)
└── aaa
   └── bbb(rb)
"
        .trim();
        fmter.fmt(&mut root).unwrap();
        println!("{}", std::str::from_utf8(&fmter.w).unwrap().trim());
        assert_eq!(std::str::from_utf8(&fmter.w).unwrap().trim(), ans);
    }
}
