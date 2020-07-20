use crate::error::Result;
use crate::script::{ScriptInfo, ScriptName};
use colored::{Color, Colorize};
use std::io::Write;

#[derive(Default)]
pub struct ListOptions {
    pub show_hidden: bool,
    pub long: bool,
}

pub fn fmt_meta<W: Write>(
    w: &mut W,
    meta: &ScriptInfo,
    is_last: bool,
    opt: &ListOptions,
) -> Result<()> {
    if opt.long {
        if is_last {
            write!(w, "{}", " *".color(Color::Yellow).bold())?;
        } else {
            write!(w, "  ")?;
        }

        let exex_time = if let Some(t) = &meta.exec_time {
            t.to_string()
        } else {
            "Never".to_owned()
        };
        write!(
            w,
            "{}\t{}\t{}\t{}\n",
            meta.ty.to_string().color(meta.ty.color()).bold(),
            meta.name,
            meta.edit_time,
            exex_time
        )?;
    } else {
        let msg = if is_last {
            meta.file_name().underline()
        } else {
            meta.file_name().normal()
        };
        write!(w, "{}", msg.bold().color(meta.ty.color()))?;
    }
    Ok(())
}
pub fn fmt_list<W: Write>(
    w: &mut W,
    scripts: impl IntoIterator<Item = ScriptInfo>,
    opt: &ListOptions,
) -> Result<()> {
    let mut scripts: Vec<_> = scripts
        .into_iter()
        .filter(|s| !s.hidden || opt.show_hidden)
        .collect();
    scripts.sort_by_key(|m| m.name.clone());
    let last_index = match scripts
        .iter()
        .enumerate()
        .max_by_key(|(_, s)| s.last_time())
    {
        Some((i, _)) => i,
        None => return Ok(()),
    };

    let mut anonymous_printed = !opt.long;
    if opt.long {
        writeln!(w, "type\tname\tlast edit time\tlast execute time")?;
    }
    for (i, meta) in scripts.iter().enumerate() {
        if i != 0 && !opt.long {
            write!(w, "  ")?;
        }
        if let (false, ScriptName::Anonymous(_)) = (anonymous_printed, &meta.name) {
            anonymous_printed = true;
            writeln!(w, "--- Anonymous ---")?;
        }
        fmt_meta(w, meta, last_index == i, opt)?;
    }
    writeln!(w, "")?;
    Ok(())
}
