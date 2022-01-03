use super::{
    exec_time_str, extract_help, style, tree, DisplayIdentStyle, DisplayStyle, Grid, Grouping,
    ListOptions,
};
use crate::error::Result;
use crate::query::do_list_query;
use crate::script::ScriptInfo;
use crate::script_repo::ScriptRepo;
use crate::tag::Tag;
use crate::util::get_display_type;
use colored::{Color, Colorize};
use fxhash::FxHashMap as HashMap;
use prettytable::{cell, format, row, Cell, Row, Table};
use std::cmp::Reverse;
use std::fmt::Write as FmtWrite;
use std::hash::Hash;
use std::io::Write;

type ListOptionWithOutput = ListOptions<Table, Grid>;

fn ident_string(style: &DisplayIdentStyle, ty: &str, script: &ScriptInfo) -> String {
    match style {
        DisplayIdentStyle::Normal => format!("{}({})", script.name, ty),
        DisplayIdentStyle::File => script.file_path_fallback().to_string_lossy().to_string(),
        DisplayIdentStyle::Name => script.name.to_string(),
        DisplayIdentStyle::NameAndFile => format!(
            "{}({})",
            script.name.to_string(),
            script.file_path_fallback().to_string_lossy().to_string()
        ),
    }
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
}

fn convert_opt<T>(opt: ListOptions, t: T) -> ListOptions<Table, T> {
    ListOptions {
        display_style: match opt.display_style {
            DisplayStyle::Short(style, _) => DisplayStyle::Short(style, t),
            DisplayStyle::Long(_) => {
                let mut table = Table::new();
                if opt.grouping != Grouping::Tree {
                    table.set_titles(Row::new(TITLE.iter().map(|t| cell!(c->t)).collect()));
                }
                table.set_format(*format::consts::FORMAT_CLEAN);
                DisplayStyle::Long(table)
            }
        },
        grouping: opt.grouping,
        queries: opt.queries,
        plain: opt.plain,
    }
}
fn extract_table<U>(opt: ListOptions<Table, U>) -> Option<Table> {
    match opt.display_style {
        DisplayStyle::Short(..) => None,
        DisplayStyle::Long(table) => Some(table),
    }
}
pub fn fmt_meta(
    script: &ScriptInfo,
    is_latest: bool,
    opt: &mut ListOptionWithOutput,
) -> Result<()> {
    let ty = get_display_type(&script.ty);
    let color = ty.color();
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
            let ty_txt = style(opt.plain, ty.display(), |s| s.color(color).bold());

            let mut buff = String::new();
            let help_msg = extract_help(&mut buff, script);

            let row =
                row![name_txt, c->ty_txt, c->script.write_time, c->exec_time_str(script), help_msg];
            table.add_row(row);
        }
        DisplayStyle::Short(ident_style, grid) => {
            let mut display_str = String::new();
            let mut width = 0;
            if is_latest && !opt.plain {
                width += 1;
                write!(display_str, "{}", "*".color(Color::Yellow).bold())?;
            }
            let ident = ident_string(ident_style, &*ty.display(), script);
            let ident = style(opt.plain, ident, |s| {
                let s = s.color(color).bold();
                if is_latest {
                    s.underline()
                } else {
                    s
                }
            });
            width += ident.chars().count();
            write!(display_str, "{}", ident)?;
            grid.add(display_str, width);
        }
    }
    Ok(())
}
const TITLE: &[&str] = &["name", "type", "write", "execute", "help message"];
pub async fn fmt_list<W: Write>(
    w: &mut W,
    script_repo: &mut ScriptRepo,
    opt: ListOptions,
) -> Result<()> {
    let latest_script_id = script_repo.latest_mut(1, false).map_or(-1, |s| s.id);

    let scripts_iter = do_list_query(script_repo, &opt.queries)
        .await?
        .into_iter()
        .map(|e| &*e.into_inner());
    let len = scripts_iter.len();

    let final_table: Option<Table>;
    match opt.grouping {
        Grouping::None => {
            let mut opt = convert_opt(opt, Grid::new(len));
            let scripts: Vec<_> = scripts_iter.collect();
            fmt_group(w, scripts, latest_script_id, &mut opt)?;
            final_table = extract_table(opt);
        }
        Grouping::Tree => {
            let mut opt = convert_opt(opt, &mut *w);
            let scripts: Vec<_> = scripts_iter.collect();
            tree::fmt(scripts, latest_script_id, &mut opt)?;
            final_table = extract_table(opt);
        }
        Grouping::Tag => {
            let mut opt = convert_opt(opt, Grid::new(len));
            let mut script_map: HashMap<TagsKey, Vec<&ScriptInfo>> = HashMap::default();
            for script in scripts_iter {
                let key = TagsKey::new(script.tags.iter().cloned());
                let v = script_map.entry(key).or_default();
                v.push(script);
            }

            let mut scripts: Vec<_> = script_map.into_iter().collect();

            // NOTE: 以群組中執行次數的最大值排序
            scripts.sort_by_key(|(_, v)| v.iter().map(|s| s.exec_count).max());

            for (tags, scripts) in scripts.into_iter() {
                if !opt.grouping.is_none() {
                    let tags_txt = style(opt.plain, tags.to_string(), |s| s.dimmed().italic());
                    match &mut opt.display_style {
                        DisplayStyle::Long(table) => {
                            table.add_row(Row::new(vec![Cell::new(&tags_txt.to_string())]));
                        }
                        DisplayStyle::Short(_, _) => {
                            writeln!(w, "{}", tags_txt)?;
                        }
                    }
                }
                fmt_group(w, scripts, latest_script_id, &mut opt)?;
            }
            final_table = extract_table(opt);
        }
    }
    if let Some(table) = final_table {
        table.print(w)?;
    }
    Ok(())
}

fn fmt_group<W: Write>(
    w: &mut W,
    mut scripts: Vec<&ScriptInfo>,
    latest_script_id: i64,
    opt: &mut ListOptionWithOutput,
) -> Result<()> {
    scripts.sort_by_key(|s| Reverse(s.last_time()));
    for script in scripts.into_iter() {
        let is_latest = script.id == latest_script_id;
        fmt_meta(script, is_latest, opt)?;
    }
    match &mut opt.display_style {
        DisplayStyle::Short(_, grid) => {
            let width = console::Term::stdout().size().1 as usize;
            let grid_display = grid.fit_into_width(width);
            write!(w, "{}", grid_display)?;
            drop(grid_display);
            grid.clear();
        }
        _ => (),
    }
    Ok(())
}
