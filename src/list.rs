use crate::error::{Contextabl, Error, Result};
use crate::script::{ScriptInfo, ScriptName};
use std::io::Write;

#[derive(Default)]
pub struct ListOptions {
    pub show_hidden: bool,
    pub long: bool,
}

pub fn fmt_meta<W: Write>(w: &mut W, meta: &ScriptInfo, opt: &ListOptions) -> Result<()> {
    if opt.long {
        let exex_time = if let Some(t) = &meta.exec_time {
            t.to_string()
        } else {
            "Never".to_owned()
        };
        write!(
            w,
            "{}\t{}\t{}\t{}",
            meta.ty, meta.name, meta.edit_time, exex_time
        )?;
    } else {
        write!(w, "{}\t{}", meta.ty, meta.name)?;
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

    let mut anonymous_printed = false;
    if opt.long {
        writeln!(w, "name\ttype\tlast edit time\tlast execute time")?;
    }
    for (i, meta) in scripts.iter().enumerate() {
        if let (false, ScriptName::Anonymous(_)) = (anonymous_printed, &meta.name) {
            anonymous_printed = true;
            writeln!(w, "--- Anonymous ---")?;
        }
        if i == last_index {
            write!(w, " *")?;
        } else {
            write!(w, "  ")?;
        }
        fmt_meta(w, meta, opt)?;
        write!(w, "\n")?;
    }
    Ok(())
}
