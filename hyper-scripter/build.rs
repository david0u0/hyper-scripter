#[path = "src/migration/mod.rs"]
mod migration;

#[tokio::main]
async fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let dir = format!("{}/db_example", manifest);
    let file = format!("{}/.script_info.db", dir);

    let _ = std::fs::remove_dir_all(&dir);

    std::fs::create_dir(dir).unwrap();
    migration::do_migrate(&file).await.unwrap();
    println!("cargo:rustc-env=DATABASE_URL=sqlite:{}", file);
}
