use crate::error::{Contextabl, Error, Result};
use crate::script::{ScriptMeta, ScriptName};
use std::io::Write;

#[derive(Default)]
pub struct ListOptions {
    pub show_hidden: bool,
}

pub fn list_str(
    scripts: impl IntoIterator<Item = ScriptMeta>,
    opt: &ListOptions,
) -> Result<String> {
    let mut v = Vec::<u8>::new();
    fmt_list(&mut v, scripts, opt)?;
    Ok(std::str::from_utf8(&v)?.to_owned())
}
pub fn fmt_meta<W: Write>(w: &mut W, meta: &ScriptMeta, opt: &ListOptions) -> Result<()> {
    if !opt.show_hidden && meta.hidden {
        return Ok(());
    }
    match &meta.name {
        ScriptName::Anonymous(id) => write!(w, ".{}", id)?,
        ScriptName::Named(name) => write!(w, "{}", name)?,
    }
    let exex_time = if let Some(t) = &meta.exec_time {
        t.to_string()
    } else {
        "Never".to_owned()
    };
    write!(w, "\t{}\t{}\n", meta.edit_time, exex_time)?;
    Ok(())
}
pub fn fmt_list<W: Write>(
    w: &mut W,
    scripts: impl IntoIterator<Item = ScriptMeta>,
    opt: &ListOptions,
) -> Result<()> {
    let mut scripts: Vec<_> = scripts.into_iter().collect();
    scripts.sort_by_key(|m| m.name.clone());
    let mut anonymous_printed = false;
    writeln!(w, "name\tlast edit time\tlast executed time")?;
    for meta in scripts.iter() {
        if let (false, ScriptName::Anonymous(_)) = (anonymous_printed, &meta.name) {
            anonymous_printed = true;
            writeln!(w, "--- Anonymous ---")?;
        }
        fmt_meta(w, meta, opt)?;
    }
    Ok(())
}
