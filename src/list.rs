use crate::error::{Contextabl, Error, Result};
use crate::script::ScriptMeta;
use std::io::Write;

pub struct ListOptions {}

pub fn list_str(
    scripts: impl IntoIterator<Item = ScriptMeta>,
    opt: &ListOptions,
) -> Result<String> {
    let mut v = Vec::<u8>::new();
    fmt_list(&mut v, scripts, opt)?;
    Ok(std::str::from_utf8(&v)?.to_owned())
}
pub fn fmt_list<W: Write>(
    w: &mut W,
    scripts: impl IntoIterator<Item = ScriptMeta>,
    opt: &ListOptions,
) -> Result<()> {
    for script in scripts.into_iter() {
        writeln!(w, "{:?}", script.name)?;
    }
    Ok(())
}
