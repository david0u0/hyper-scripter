#[path = "src/migration/mod.rs"]
mod migration;

#[tokio::main]
async fn main() {
    let _ = std::fs::remove_dir_all("db_example");
    std::fs::create_dir("db_example").unwrap();
    migration::do_migrate("db_example").await.unwrap();
    println!("cargo:rustc-env=DATABASE_URL=sqlite:db_example/script_info.db");
}
