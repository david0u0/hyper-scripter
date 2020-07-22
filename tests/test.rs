use instant_scripter::path;
use std::process::Command;

fn setup() {
    path::set_path_from_sys().unwrap();
    std::fs::remove_dir_all(path::get_path()).unwrap();
}
fn run(args: &[&str]) -> String {
    let mut cmd = Command::new("./target/debug/instant_scripter");
    let out = cmd.args(args).output().unwrap();
    let s = std::str::from_utf8(&out.stdout).unwrap();
    s.trim().to_owned()
}

#[test]
fn test_main() {
    env_logger::init();
    setup();
    let msg = "你好，腳本狂！";
    run(&["e", "-c", &format!("echo \"{}\"", msg)]);
    let out_msg = run(&["-"]);
    assert_eq!(msg, out_msg);

    run(&[
        "e",
        "test_js",
        "-x",
        "js",
        "-c",
        &format!("console.log(\"{}\")", msg),
    ]);
    let out_msg = run(&["-"]);
    assert_eq!(msg, out_msg);
}
