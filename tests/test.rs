use instant_scripter::path;
use std::process::Command;

fn setup() {
    path::set_path_from_sys().unwrap();
    std::fs::remove_dir_all(path::get_path()).unwrap();
}
fn run(args: &[&str]) -> Result<String, i32> {
    let mut cmd = Command::new("./target/debug/instant_scripter");
    let out = cmd.args(args).output().unwrap();
    let status = out.status.code().unwrap_or_default();
    if status != 0 {
        Err(status)
    } else {
        let s = std::str::from_utf8(&out.stdout).unwrap();
        Ok(s.trim().to_owned())
    }
}

#[test]
fn test_main() {
    env_logger::init();
    setup();
    let msg = "你好，腳本狂！";
    run(&["e", "-c", &format!("echo \"{}\"", msg)]).unwrap();
    let out_msg = run(&["-"]).unwrap();
    assert_eq!(msg, out_msg);

    run(&[
        "-t",
        "super_tag",
        "e",
        "test_js",
        "-x",
        "js",
        "-c",
        &format!("console.log(\"{}\")", msg),
    ])
    .unwrap();
    run(&["-"]).expect_err("標籤沒有篩選掉不該出現的腳本！");

    run(&["-t", "super_tag", "-"]).unwrap();
}
