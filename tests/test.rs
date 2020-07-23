use instant_scripter::path;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

fn setup() {
    let _ = env_logger::try_init();
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
            println!("{}", line);
            out_str.push(line);
        });

    let status = child.wait().unwrap();
    if status.success() {
        Ok(out_str.join("\n"))
    } else {
        Err(status.code().unwrap_or_default())
    }
}

const MSG: &'static str = "你好，腳本管理員！";
const MSG_JS: &'static str = "你好，腳本管理員！.js";
fn test_create_and_run() {
    setup();
    run(&["e", "-c", &format!("echo \"{}\"", MSG)]).unwrap();
    assert_eq!(MSG, run(&["-"]).unwrap());

    run(&[
        "-t",
        "super_tag,hide",
        "e",
        "test_js",
        "-x",
        "js",
        "-c",
        &format!("console.log(\"{}\")", MSG_JS),
    ])
    .unwrap();
    run(&["tesjs"]).expect_err("標籤沒有篩選掉不該出現的腳本！");
    assert_eq!(MSG_JS, run(&["-t", "super_tag", "-"]).unwrap());

    assert_eq!(MSG, run(&[".1"]).expect("標籤篩選把舊的腳本搞爛了！"));
}

fn test_mv() {
    setup();
    run(&["e", "-x", "js", "-c", &format!("echo \"{}\"", MSG)]).unwrap();
    run(&["-"]).expect_err("用 nodejs 執行 echo ……？");

    run(&["mv", "1", "-x", "sh"]).unwrap();
    assert_eq!(MSG, run(&["-"]).unwrap());
    assert!(
        path::get_path().join(".anonymous/1.sh").exists(),
        "改腳本類型失敗"
    );
    assert!(
        !path::get_path().join(".anonymous/1.js").exists(),
        "改了腳本類型舊檔案還留著？"
    );
}

const TALKER: &'static str = "--腳本小子";
const APPEND: &'static str = "第二行";
fn test_args() {
    setup();
    run(&[
        "e",
        "test-with-args",
        "-c",
        &format!("echo -e \"$1：{}\n$2\"", MSG),
    ])
    .unwrap();
    assert_eq!(
        format!("{}：{}\n{}", TALKER, MSG, APPEND),
        run(&["-", TALKER, APPEND]).unwrap()
    );
}
fn test_exact() {
    setup();
    run(&["e", "test-exact", "-c", "echo 'test exact!'"]).unwrap();
    run(&["tesct"]).expect("模糊搜不到東西！");
    run(&["=tesct"]).expect_err("打錯名字卻還搜得到！");
    run(&["=test-exact"]).expect("打完整名字卻搜不到！");
}
#[test]
fn test_main() {
    test_create_and_run();
    test_mv();
    test_args();
    test_exact();
}
