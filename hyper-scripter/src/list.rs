use crate::config::Config;
use crate::error::Result;
use crate::query::{do_list_query, ListQuery};
use crate::script::{ScriptInfo, ScriptName};
use crate::script_repo::ScriptRepo;
use crate::tag::Tag;
use colored::{Color, ColoredString, Colorize};
use prettytable::{cell, format, row, Cell, Row, Table};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::Write;

#[derive(PartialEq, Eq)]
struct TagsKey(Vec<Tag>);
impl Hash for TagsKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for tag in self.0.iter() {
            tag.as_ref().hash(state);
        }
    }
}
impl std::fmt::Display for TagsKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() == 0 {
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
        while self_slice.len() > 0 && other_slice.len() > 0 {
            let (t1, t2) = (&self_slice[0], &other_slice[0]);
            let cmp = t1.partial_cmp(t2);
            if cmp != Some(std::cmp::Ordering::Equal) {
                return cmp;
            }
            self_slice = &self_slice[1..self_slice.len()];
            other_slice = &other_slice[1..other_slice.len()];
        }
        self.0.len().partial_cmp(&other.0.len())
    }
    fn cmp(&self, other: &TagsKey) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DisplayIdentStyle {
    File,
    Name,
    Normal,
}
#[derive(Debug, Eq, PartialEq)]
pub enum DisplayStyle<T, U> {
    Short(DisplayIdentStyle, U),
    Long(T),
}
impl<T, U> DisplayStyle<T, U> {
    pub fn is_long(&self) -> bool {
        if let DisplayStyle::Long(_) = self {
            true
        } else {
            false
        }
    }
}
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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
impl<T: AsRef<str>> From<T> for Grouping {
    fn from(s: T) -> Self {
        match s.as_ref() {
            "tag" => Grouping::Tag,
            "tree" => {
                unimplemented!();
                // Grouping::Tree
            }
            "none" => Grouping::None,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub struct ListOptions<'a, T = (), U = ()> {
    pub grouping: Grouping,
    pub queries: &'a [ListQuery],
    pub plain: bool,
    pub display_style: DisplayStyle<T, U>,
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
fn get_color(script: &ScriptInfo) -> Result<Color> {
    let c = Config::get()?.get_script_conf(&script.ty)?.color.as_str();
    Ok(Color::from(c))
}
pub fn fmt_meta<W: Write>(
    script: &ScriptInfo,
    is_last: bool,
    opt: &mut ListOptions<Table, &mut W>,
) -> Result<()> {
    let color = get_color(script)?;
    match &mut opt.display_style {
        DisplayStyle::Long(table) => {
            let last_txt = if is_last && !opt.plain {
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
            let exec_time_txt = match &script.exec_time {
                Some(t) => t.to_string(),
                None => "Never".to_owned(),
            };
            let row = row![name_txt, c->ty_txt, script.write_time, exec_time_txt];
            table.add_row(row);
        }
        DisplayStyle::Short(ident_style, w) => {
            if is_last && !opt.plain {
                write!(w, "{}", "*".color(Color::Yellow).bold())?;
            }
            let ident = match ident_style {
                DisplayIdentStyle::Normal => format!("{}({})", script.name, script.ty),
                DisplayIdentStyle::File => script.file_path()?.to_string_lossy().to_string(),
                DisplayIdentStyle::Name => script.name.to_string(),
            };
            let ident = style(opt.plain, ident, |s| {
                let s = s.color(color).bold();
                if is_last {
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
const TITLE: &[&str] = &["name", "category", "last write time", "last execute time"];
pub fn fmt_list<'a, W: Write>(
    w: &mut W,
    script_repo: &mut ScriptRepo,
    opt: &ListOptions,
) -> Result<()> {
    let mut opt = convert_opt(w, opt);

    let latest_script_name = match script_repo.latest_mut(1) {
        Some(script) => script.name.clone().into_static(),
        None => return Ok(()),
    };

    if let DisplayStyle::Long(table) = &mut opt.display_style {
        table.set_format(*format::consts::FORMAT_CLEAN);
        table.set_titles(Row::new(TITLE.iter().map(|t| cell!(c->t)).collect()));
    }

    let scripts_iter = do_list_query(script_repo, &opt.queries)?
        .into_iter()
        .map(|e| &*e.into_inner());

    if opt.grouping.is_none() {
        let scripts: Vec<_> = scripts_iter.collect();
        fmt_group(scripts, &latest_script_name, &mut opt)?;
    } else {
        // TODO: 樹狀
        let mut script_map: HashMap<TagsKey, Vec<&ScriptInfo>> = HashMap::default();
        for script in scripts_iter {
            let key = TagsKey::new(script.tags.iter().map(|t| t.clone()));
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
                        write!(w, "{}\n", tags_txt)?;
                    }
                }
            }
            fmt_group(scripts, &latest_script_name, &mut opt)?;
        }
    }
    if let DisplayStyle::Long(table) = &mut opt.display_style {
        table.print(w)?;
    }
    Ok(())
}

fn fmt_group<W: Write>(
    mut scripts: Vec<&ScriptInfo>,
    latest_script_name: &ScriptName,
    opt: &mut ListOptions<Table, &mut W>,
) -> Result<()> {
    scripts.sort_by(|s1, s2| s2.last_time().cmp(&s1.last_time()));
    for script in scripts {
        if let DisplayStyle::Short(_, w) = &mut opt.display_style {
            write!(w, "  ")?;
        }
        let is_latest = &script.name == latest_script_name;
        fmt_meta(script, is_latest, opt)?;
    }
    if let DisplayStyle::Short(_, w) = &mut opt.display_style {
        write!(w, "\n")?;
    }
    Ok(())
}
