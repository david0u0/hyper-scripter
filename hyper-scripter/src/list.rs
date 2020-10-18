use crate::config::Config;
use crate::error::{Error, FormatCode::Grouping as GroupCode, Result};
use crate::query::{do_list_query, ListQuery};
use crate::script::{ScriptInfo, ScriptName};
use crate::script_repo::ScriptRepo;
use crate::tag::Tag;
use colored::{Color, Colorize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::str::FromStr;

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
pub enum DisplayScriptIdent {
    File,
    Name,
    Normal,
}
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DisplayStyle {
    Short(DisplayScriptIdent),
    Long,
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
impl FromStr for Grouping {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "tag" => Grouping::Tag,
            "tree" => {
                unimplemented!();
                // Grouping::Tree
            }
            "none" => Grouping::None,
            _ => return Err(Error::Format(GroupCode, s.to_owned())),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ListOptions<'a> {
    pub grouping: Grouping,
    pub queries: &'a [ListQuery],
    pub plain: bool,
    pub display_style: DisplayStyle,
}

pub fn fmt_meta<W: Write>(
    w: &mut W,
    script: &ScriptInfo,
    is_last: bool,
    opt: &ListOptions,
) -> Result<()> {
    let color = Config::get()?.get_script_conf(&script.ty)?.color.as_str();
    match opt.display_style {
        DisplayStyle::Long => {
            if is_last && !opt.plain {
                write!(w, "{}", " *".color(Color::Yellow).bold())?;
            } else {
                write!(w, "  ")?;
            }

            let exex_time = match &script.exec_time {
                Some(t) => t.to_string(),
                None => "Never".to_owned(),
            };
            let mut label: colored::ColoredString =
                format!("{}\t{}", script.ty, script.name).normal();
            if !opt.plain {
                label = label.color(color).bold();
            }
            write!(
                w,
                "{}\t{}\t{}\t{}\n",
                label, script.created_time, script.read_time, exex_time
            )?;
        }
        DisplayStyle::Short(ident) => {
            if is_last && !opt.plain {
                write!(w, "{}", "*".color(Color::Yellow).bold())?;
            }
            let msg = match ident {
                DisplayScriptIdent::Normal => format!("{}({})", script.name, script.ty),
                DisplayScriptIdent::File => script.file_path()?.to_string_lossy().to_string(),
                DisplayScriptIdent::Name => script.name.to_string(),
            };
            if !opt.plain {
                let mut msg = msg.bold().color(color);
                if is_last {
                    msg = msg.underline()
                }
                write!(w, "{}", msg)?;
            } else {
                write!(w, "{}", msg)?;
            }
        }
    }
    Ok(())
}
pub fn fmt_list<'a, W: Write>(
    w: &mut W,
    script_repo: &mut ScriptRepo,
    opt: &ListOptions,
) -> Result<()> {
    let latest_script_name = match script_repo.latest_mut(1) {
        Some(script) => script.name.clone().into_static(),
        None => return Ok(()),
    };

    if opt.display_style == DisplayStyle::Long {
        writeln!(
            w,
            "type\tname\tcreate time\tlast read time\tlast execute time"
        )?;
    }
    let scripts_iter = do_list_query(script_repo, &opt.queries)?
        .into_iter()
        .map(|e| &*e.into_inner());

    if opt.grouping.is_none() {
        let scripts: Vec<_> = scripts_iter.collect();
        fmt_group(w, scripts, &latest_script_name, opt)?;
        return Ok(());
    }

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
            write!(w, "{}\n", tags.to_string().dimmed().italic())?;
        }
        fmt_group(w, scripts, &latest_script_name, opt)?;
    }
    Ok(())
}

fn fmt_group<W: Write>(
    w: &mut W,
    mut scripts: Vec<&ScriptInfo>,
    latest_script_name: &ScriptName,
    opt: &ListOptions,
) -> Result<()> {
    scripts.sort_by(|s1, s2| s2.last_time().cmp(&s1.last_time()));
    for script in scripts {
        if opt.display_style != DisplayStyle::Long {
            write!(w, "  ")?;
        }
        let is_latest = &script.name == latest_script_name;
        fmt_meta(w, script, is_latest, opt)?;
    }
    write!(w, "\n")?;
    Ok(())
}
