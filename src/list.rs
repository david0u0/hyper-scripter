use crate::config::Config;
use crate::error::{Error, FormatCode, Result};
use crate::history::History;
use crate::script::{ScriptInfo, ScriptName};
use crate::tag::Tag;
use colored::{Color, Colorize};
use regex::Regex;
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
    fn partial_cmp(&self, other: &TagsKey) -> Option<std::cmp::Ordering> {
        if other.0.len() != self.0.len() {
            return self.0.len().partial_cmp(&other.0.len());
        }
        for (t1, t2) in self.0.iter().zip(other.0.iter()) {
            if t1 != t2 {
                return t1.partial_cmp(t2);
            }
        }
        Some(std::cmp::Ordering::Equal)
    }
    fn cmp(&self, other: &TagsKey) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

#[derive(Debug)]
pub struct ListPattern(Regex);
impl std::str::FromStr for ListPattern {
    type Err = Error;
    fn from_str(s: &str) -> std::result::Result<Self, Error> {
        // TODO: 好好檢查
        let s = s.replace(".", "\\.");
        let s = s.replace("*", ".*");
        let re = Regex::new(&format!("^{}$", s)).map_err(|e| {
            log::error!("正規表達式錯誤：{}", e);
            Error::Format(FormatCode::Regex, s)
        })?;
        Ok(ListPattern(re))
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
pub struct ListOptions<'a> {
    pub no_grouping: bool,
    pub pattern: &'a Option<ListPattern>,
    pub plain: bool,
    pub display_style: DisplayStyle,
}
impl<'a> ListOptions<'a> {
    fn filter(&self, script: &ScriptInfo) -> bool {
        match &self.pattern {
            Some(ListPattern(re)) => re.is_match(&script.name.to_string()),
            _ => true,
        }
    }
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

            let exex_time = match &script.get_exec_time() {
                Some(t) => t.to_string(),
                None => "Never".to_owned(),
            };
            let mut label: colored::ColoredString =
                format!("{}\t{}", script.ty, script.name).normal();
            if !opt.plain {
                label = label.color(color).bold();
            }
            write!(w, "{}\t{}\t{}\n", label, script.get_read_time(), exex_time)?;
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
pub fn fmt_list<'a, W: Write>(w: &mut W, history: &mut History, opt: &ListOptions) -> Result<()> {
    let latest_script_name = match history.latest_mut(1) {
        Some(script) => script.name.clone().into_static(),
        None => return Ok(()),
    };

    if opt.display_style == DisplayStyle::Long {
        writeln!(w, "type\tname\tlast read time\tlast execute time")?;
    }
    let script_iter = history.iter().filter(|s| opt.filter(&s));

    if opt.no_grouping {
        let scripts: Vec<_> = script_iter.collect();
        fmt_group(w, scripts, &latest_script_name, opt)?;
        return Ok(());
    }

    let mut scripts: HashMap<TagsKey, Vec<&ScriptInfo>> = HashMap::default();
    for script in script_iter {
        let key = TagsKey(script.tags.clone());
        let v = scripts.entry(key).or_default();
        v.push(script);
    }

    let mut scripts: Vec<_> = scripts.into_iter().collect();

    scripts.sort_by(|(t1, _), (t2, _)| t1.cmp(t2));
    for (tags, scripts) in scripts.into_iter() {
        if !opt.no_grouping {
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
