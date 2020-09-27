use std::env;
use std::fs::{read_dir, File};
use std::io::prelude::*;
use std::path::Path;

fn join_file(s: &str) -> String {
    let util_dir = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("util");
    util_dir.join(s).to_string_lossy().to_string()
}

fn read_all() -> std::io::Result<impl Iterator<Item = String>> {
    let dir = read_dir(join_file(""))?;
    let iter = dir
        .into_iter()
        .map(|f| f.unwrap().file_name().to_string_lossy().to_string());
    Ok(iter)
}

fn main() -> std::io::Result<()> {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("get_all_utils.rs");
    let mut file = File::create(dest)?;
    let inner = read_all()?
        .map(|path| {
            let mut splited = path.rsplitn(2, ".");
            let category = splited.next().unwrap();
            let name = splited.next().unwrap();
            format!(
                "(\"util/{}\", \"{}\", std::include_str!(\"{}\"))",
                name,
                category,
                join_file(&path)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    file.write_all(
        b"pub fn get_all() -> &'static [(&'static str, &'static str, &'static str)] {\n",
    )?;
    file.write_all(format!("    &[{}]", inner).as_bytes())?;
    file.write_all(b"}\n")?;
    Ok(())
}
