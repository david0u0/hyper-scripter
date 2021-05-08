use super::{style, time_str, tree, DisplayIdentStyle, DisplayStyle, Grouping, ListOptions};
use crate::config::Config;
use crate::error::Result;
use crate::extract_help;
use crate::query::do_list_query;
use crate::script::ScriptInfo;
use crate::script_repo::ScriptRepo;
use crate::tag::Tag;
use colored::{Color, Colorize};
use fxhash::FxHashMap as HashMap;
use prettytable::{cell, format, row, Cell, Row, Table};
use std::hash::Hash;
use std::io::Write;

fn ident_string(style: &DisplayIdentStyle, script: &ScriptInfo) -> Result<String> {
    Ok(match style {
        DisplayIdentStyle::Normal => format!("{}({})", script.name, script.ty),
        DisplayIdentStyle::File => script.file_path()?.to_string_lossy().to_string(),
        DisplayIdentStyle::Name => script.name.to_string(),
    })
}

#[derive(PartialEq, Eq, Hash)]
struct TagsKey(Vec<Tag>);
impl std::fmt::Display for TagsKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            write!(f, "(no tag)")?;
            return Ok(());
        }
        write!(f, "[")?;
        let mut first = true;
        for tag in &self.0 {
            if !first {
                write!(f, " ")?;
            }
            first = false;
            write!(f, "#{}", AsRef::<str>::as_ref(tag))?;
        }
        write!(f, "]")?;
        Ok(())
    }
}
impl TagsKey {
    fn new(tags: impl Iterator<Item = Tag>) -> Self {
        let mut tags: Vec<_> = tags.collect();
        tags.sort();
        TagsKey(tags)
    }
    fn partial_cmp(&self, other: &TagsKey) -> Option<std::cmp::Ordering> {
        let mut self_slice: &[_] = &self.0;
        let mut other_slice: &[_] = &other.0;
        while !self_slice.is_empty() && !other_slice.is_empty() {
            let (t1, t2) = (&self_slice[0], &other_slice[0]);
            let cmp = t1.partial_cmp(t2);
            if cmp != Some(std::cmp::Ordering::Equal) {
                return cmp;
            }
            self_slice = &self_slice[1..];
            other_slice = &other_slice[1..];
        }
        self.0.len().partial_cmp(&other.0.len())
    }
    fn cmp(&self, other: &TagsKey) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

fn convert_opt<'a, W: Write>(
    w: &'a mut W,
    opt: &ListOptions<'a>,
) -> ListOptions<'a, Table, &'a mut W> {
    ListOptions {
        display_style: match opt.display_style {
            DisplayStyle::Short(t, _) => DisplayStyle::Short(t, w),
            DisplayStyle::Long(_) => DisplayStyle::Long(Table::new()),
        },
        grouping: opt.grouping,
        queries: opt.queries,
        plain: opt.plain,
    }
}
pub fn fmt_meta<W: Write>(
    script: &ScriptInfo,
    is_latest: bool,
    opt: &mut ListOptions<Table, &mut W>,
) -> Result<()> {
    let color = Config::get()?.get_color(&script.ty)?;
    match &mut opt.display_style {
        DisplayStyle::Long(table) => {
            let last_txt = if is_latest && !opt.plain {
                "*".color(Color::Yellow).bold()
            } else {
                " ".normal()
            };
            let name_txt = format!(
                "{} {}",
                last_txt,
                style(opt.plain, script.name.key(), |s| s.color(color).bold()),
            );
            let ty_txt = style(opt.plain, &script.ty, |s| s.color(color).bold());

            extract_help!(help_msg, script, false);
            let help_msg = help_msg.into_iter().next().unwrap_or_default();

            let row = row![name_txt, c->ty_txt, c->script.write_time, c->time_str(&script.exec_time), help_msg];
            table.add_row(row);
        }
        DisplayStyle::Short(ident_style, w) => {
            if is_latest && !opt.plain {
                write!(w, "{}", "*".color(Color::Yellow).bold())?;
            }
            let ident = ident_string(ident_style, script)?;
            let ident = style(opt.plain, ident, |s| {
                let s = s.color(color).bold();
                if is_latest {
                    s.underline()
                } else {
                    s
                }
            });
            write!(w, "{}", ident)?;
        }
    }
    Ok(())
}
const TITLE: &[&str] = &[
    "name",
    "category",
    "last write time",
    "last execute time",
    "help message",
];
pub async fn fmt_list<W: Write>(
    w: &mut W,
    script_repo: &'_ mut ScriptRepo,
    opt: &ListOptions<'_>,
) -> Result<()> {
    let mut opt = convert_opt(w, opt);

    let latest_script_id = match script_repo.latest_mut(1, false) {
        Some(script) => script.id,
        None => return Ok(()),
    };

    if let DisplayStyle::Long(table) = &mut opt.display_style {
        if opt.grouping != Grouping::Tree {
            table.set_titles(Row::new(TITLE.iter().map(|t| cell!(c->t)).collect()));
        }
        table.set_format(*format::consts::FORMAT_CLEAN);
    }

    let scripts_iter = do_list_query(script_repo, &opt.queries)
        .await?
        .into_iter()
        .map(|e| &*e.into_inner());

    match opt.grouping {
        Grouping::None => {
            let scripts: Vec<_> = scripts_iter.collect();
            fmt_group(scripts, latest_script_id, &mut opt)?;
        }
        Grouping::Tree => {
            let scripts: Vec<_> = scripts_iter.collect();
            tree::fmt(scripts, latest_script_id, &mut opt)?;
        }
        Grouping::Tag => {
            let mut script_map: HashMap<TagsKey, Vec<&ScriptInfo>> = HashMap::default();
            for script in scripts_iter {
                let key = TagsKey::new(script.tags.iter().cloned());
                let v = script_map.entry(key).or_default();
                v.push(script);
            }

            let mut scripts: Vec<_> = script_map.into_iter().collect();

            scripts.sort_by(|(t1, _), (t2, _)| t1.cmp(t2));
            for (tags, scripts) in scripts.into_iter() {
                if !opt.grouping.is_none() {
                    let tags_txt = style(opt.plain, tags.to_string(), |s| s.dimmed().italic());
                    match &mut opt.display_style {
                        DisplayStyle::Long(table) => {
                            table.add_row(Row::new(vec![
                                Cell::new(&tags_txt.to_string()).with_hspan(TITLE.len())
                            ]));
                        }
                        DisplayStyle::Short(_, w) => {
                            writeln!(w, "{}", tags_txt)?;
                        }
                    }
                }
                fmt_group(scripts, latest_script_id, &mut opt)?;
            }
        }
    }
    if let DisplayStyle::Long(table) = &mut opt.display_style {
        table.print(w)?;
    }
    Ok(())
}

fn fmt_group<W: Write>(
    mut scripts: Vec<&ScriptInfo>,
    latest_script_id: i64,
    opt: &mut ListOptions<Table, &mut W>,
) -> Result<()> {
    scripts.sort_by_key(|s| std::cmp::Reverse(s.last_time()));
    let mut scripts = scripts.iter();
    if let Some(script) = scripts.next() {
        let is_latest = script.id == latest_script_id;
        fmt_meta(script, is_latest, opt)?;
    }
    for script in scripts {
        if let DisplayStyle::Short(_, w) = &mut opt.display_style {
            write!(w, "  ")?;
        }
        let is_latest = script.id == latest_script_id;
        fmt_meta(script, is_latest, opt)?;
    }
    if let DisplayStyle::Short(_, w) = &mut opt.display_style {
        writeln!(w)?;
    }
    Ok(())
}
