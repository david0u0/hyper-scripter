use crate::error::{Contextabl, Error, Result};
use crate::script::{ScriptMeta, ScriptName};
use std::io::Write;

#[derive(Default)]
pub struct ListOptions {
    pub show_hidden: bool,
    pub long: bool,
}

pub fn fmt_meta<W: Write>(w: &mut W, meta: &ScriptMeta, opt: &ListOptions) -> Result<bool> {
    if !opt.show_hidden && meta.hidden {
        return Ok(false);
    }
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
    Ok(true)
}
pub fn fmt_list<W: Write>(
    w: &mut W,
    scripts: impl IntoIterator<Item = ScriptMeta>,
    opt: &ListOptions,
) -> Result<()> {
    let mut scripts: Vec<_> = scripts.into_iter().collect();
    scripts.sort_by_key(|m| m.name.clone());
    let mut anonymous_printed = false;
    if opt.long {
        writeln!(w, "name\ttype\tlast edit time\tlast execute time")?;
    }
    for meta in scripts.iter() {
        if let (false, ScriptName::Anonymous(_)) = (anonymous_printed, &meta.name) {
            anonymous_printed = true;
            writeln!(w, "--- Anonymous ---")?;
        }
        if fmt_meta(w, meta, opt)? {
            write!(w, "\n")?;
        }
    }
    Ok(())
}
