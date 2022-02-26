use crate::error::{Error, Result};
use shlex::Shlex;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

const SHEBANG: &str = "#!";

pub fn handle(p: &Path) -> Result<(String, Vec<String>)> {
    let file = File::open(p)?;
    let buff = BufReader::new(file);
    if let Some(first_line) = buff.lines().next() {
        let first_line = first_line?;
        if first_line.starts_with(SHEBANG) {
            let mut iter = Shlex::new(&first_line[SHEBANG.len()..]);
            if let Some(first) = iter.next() {
                return Ok((first, iter.collect()));
            }
        }
    }
    Err(Error::PermissionDenied(vec![p.to_path_buf()]))
}
