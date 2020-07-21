use crate::error::Result;
use crate::script::ScriptInfo;
use colored::{Color, Colorize};
use regex::Regex;
use std::io::Write;

#[derive(Debug)]
pub struct ListPattern(Regex);
impl std::str::FromStr for ListPattern {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // TODO: 好好檢查
        let s = s.replace(".", "\\.");
        let s = s.replace("*", ".*");
        let re = Regex::new(&format!("^{}$", s)).map_err(|e| e.to_string())?;
        Ok(ListPattern(re))
    }
}
pub struct ListOptions<'a> {
    pub long: bool,
    pub pattern: &'a Option<ListPattern>,
}
impl<'a> ListOptions<'a> {
    fn filter(&self, script: &ScriptInfo) -> bool {
        if let Some(ListPattern(re)) = &self.pattern {
            re.is_match(&script.name.to_string())
        } else {
            true
        }
    }
}

pub fn fmt_meta<W: Write>(
    w: &mut W,
    script: &ScriptInfo,
    is_last: bool,
    opt: &ListOptions,
) -> Result<()> {
    if opt.long {
        if is_last {
            write!(w, "{}", " *".color(Color::Yellow).bold())?;
        } else {
            write!(w, "  ")?;
        }

        let exex_time = if let Some(t) = &script.exec_time {
            t.to_string()
        } else {
            "Never".to_owned()
        };
        write!(
            w,
            "{}\t{}\t{}\n",
            format!("{}\t{}", script.ty, script.name)
                .color(script.ty.color())
                .bold(),
            script.edit_time,
            exex_time
        )?;
    } else {
        let msg = if is_last {
            script.file_name().underline()
        } else {
            script.file_name().normal()
        };
        write!(w, "{}", msg.bold().color(script.ty.color()))?;
    }
    Ok(())
}
pub fn fmt_list<W: Write>(
    w: &mut W,
    scripts: impl IntoIterator<Item = ScriptInfo>,
    opt: &ListOptions,
) -> Result<()> {
    let mut scripts: Vec<_> = scripts.into_iter().filter(|s| opt.filter(s)).collect();
    scripts.sort_by_key(|m| m.name.clone());
    let last_index = match scripts
        .iter()
        .enumerate()
        .max_by_key(|(_, s)| s.last_time())
    {
        Some((i, _)) => i,
        None => return Ok(()),
    };

    if opt.long {
        writeln!(w, "type\tname\tlast edit time\tlast execute time")?;
    }
    for (i, script) in scripts.iter().enumerate() {
        if i != 0 && !opt.long {
            write!(w, "  ")?;
        }
        fmt_meta(w, script, last_index == i, opt)?;
    }
    writeln!(w, "")?;
    Ok(())
}
