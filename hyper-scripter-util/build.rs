use std::env;
use std::fs::{read_dir, File};
use std::io::prelude::*;
use std::path::Path;

fn join_file(s: &str, base: Option<&str>) -> String {
    let dir =
        Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join(base.unwrap_or_default());
    dir.join(s).to_string_lossy().to_string()
}

fn join_util(s: &str) -> String {
    join_file(s, Some("util"))
}

fn read_all() -> std::io::Result<impl Iterator<Item = String>> {
    let dir = read_dir(join_util(""))?;
    let iter = dir
        .into_iter()
        .map(|f| f.unwrap().file_name().to_string_lossy().to_string());
    Ok(iter)
}

fn read_hidden() -> std::io::Result<Vec<String>> {
    let hidden_list = join_file("hidden_list", None);
    let mut file = File::open(hidden_list)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content.split('\n').map(|s| s.trim().to_owned()).collect())
}

fn main() -> std::io::Result<()> {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("get_all_utils.rs");
    let hidden_list = read_hidden()?;
    let mut file = File::create(dest)?;
    let inner = read_all()?
        .map(|path| {
            let mut splited = path.rsplitn(2, '.');
            let ty = splited.next().unwrap();
            let name = splited.next().unwrap();
            let hidden = hidden_list.iter().any(|s| s == name);
            format!(
                "
                RawUtil {{
                    name: \"util/{}\",
                    ty: \"{}\",
                    content: std::include_str!(r\"{}\"),
                    is_hidden: {},
                }}
                ",
                name,
                ty,
                join_util(&path),
                hidden
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    file.write_all(
        b"pub struct RawUtil {
            pub name: &'static str,
            pub ty: &'static str,
            pub content: &'static str,
            pub is_hidden: bool,
        }",
    )?;
    file.write_all(b"pub fn get_all() -> &'static [RawUtil] {\n")?;
    file.write_all(format!("    &[{}]", inner).as_bytes())?;
    file.write_all(b"}\n")?;
    Ok(())
}
