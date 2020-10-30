#[path = "src/migration/mod.rs"]
mod migration;

use std::path::Path;

#[tokio::main]
async fn main() {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let file = Path::new(&out_dir).join(".script_history.db");

    migration::do_migrate(&file).await.unwrap();
    println!(
        "cargo:rustc-env=DATABASE_URL=sqlite:{}",
        file.to_string_lossy()
    );
}
