use instant_scripter::path;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

fn setup() {
    path::set_path_from_sys().unwrap();
    std::fs::remove_dir_all(path::get_path()).unwrap();
}
fn run(args: &[&str]) -> Result<String, i32> {
    let mut cmd = Command::new("./target/debug/instant_scripter");
    let mut child = cmd.args(args).stdout(Stdio::piped()).spawn().unwrap();
    let stdout = child.stdout.as_mut().unwrap();
    let mut out_str = vec![];
    let reader = BufReader::new(stdout);
    reader
        .lines()
        .filter_map(|line| line.ok())
        .for_each(|line| {
            out_str.push(line);
        });

    let status = child.wait().unwrap();
    if status.success() {
        Ok(out_str.join("\n"))
    } else {
        Err(status.code().unwrap_or_default())
    }
}

#[test]
fn test_main() {
    env_logger::init();
    setup();
    let msg = "你好，腳本狂！";
    let msg_js = "你好，腳本狂！.js";
    run(&["e", "-c", &format!("echo \"{}\"", msg)]).unwrap();
    assert_eq!(msg, run(&["-"]).unwrap());

    run(&[
        "-t",
        "super_tag",
        "e",
        "test_js",
        "-x",
        "js",
        "-c",
        &format!("console.log(\"{}\")", msg_js),
    ])
    .unwrap();
    run(&["tesjs"]).expect_err("標籤沒有篩選掉不該出現的腳本！");
    assert_eq!(msg_js, run(&["-t", "super_tag", "-"]).unwrap());

    assert_eq!(msg, run(&[".1"]).unwrap());
}
