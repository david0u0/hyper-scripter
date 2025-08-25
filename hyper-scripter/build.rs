#[path = "src/migration/mod.rs"]
mod migration;

use std::path::Path;

#[tokio::main]
async fn main() {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let file = Path::new(&out_dir).join(".script_info.db");
    let file_str = file.to_string_lossy();

    migration::do_migrate_with_pre_sql(&file, None)
        .await
        .expect(&format!("error creating db file {file_str}"));
    println!("cargo:rustc-env=DATABASE_URL=sqlite:{}", file_str);
}
