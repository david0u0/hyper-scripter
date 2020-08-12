use instant_scripter::path;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard};

lazy_static::lazy_static! {
    static ref LOCK: Mutex<()> = Mutex::new(());
}

fn setup<'a>() -> MutexGuard<'a, ()> {
    let guard = LOCK.lock().unwrap();
    let _ = env_logger::try_init();
    path::set_path_from_sys().unwrap();
    match std::fs::remove_dir_all(path::get_path()) {
        Ok(_) => (),
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                panic!("重整測試用資料夾失敗了……")
            }
        }
    }
    guard
}
fn check_exist(p: &str) -> bool {
    path::get_path().join(p).exists()
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

const MSG: &'static str = "你好，腳本人！";
const MSG_JS: &'static str = "你好，腳本人！.js";
#[test]
fn test_tags() {
    let _g = setup();
    run(&["e", "fast", &format!("echo \"{}\"", MSG)]).unwrap();
    assert_eq!(MSG, run(&["-"]).unwrap());

    run(&[
        "-t",
        "super_tag,hide",
        "e",
        "test_js",
        "-c",
        "js",
        "fast",
        &format!("console.log(\"{}\")", MSG_JS),
    ])
    .unwrap();
    run(&["tesjs"]).expect_err("標籤沒有篩選掉不該出現的腳本！");
    assert_eq!(MSG_JS, run(&["-t", "super_tag", "-"]).unwrap());

    assert_eq!(MSG, run(&[".1"]).expect("標籤篩選把舊的腳本搞爛了！"));

    run(&["tesjs"]).expect_err("標籤沒有篩選掉不該出現的腳本！可能是上上個操作把設定檔寫爛了");
    run(&["tags", "all"]).unwrap();
    run(&["tesjs"]).expect("沒吃到設定檔的標籤？");
}

#[test]
fn test_mv() {
    let _g = setup();
    run(&["e", ".", "-c", "js", "fast", &format!("echo \"{}\"", MSG)]).unwrap();
    run(&["-"]).expect_err("用 nodejs 執行 echo ……？");

    run(&["mv", "1", "-c", "sh"]).unwrap();
    assert_eq!(MSG, run(&["-"]).unwrap());
    assert!(check_exist(".anonymous/1.sh"), "改腳本類型失敗");
    assert!(
        !check_exist(".anonymous/1.js"),
        "改了腳本類型舊檔案還留著？"
    );

    run(&["mv", "1", "-t", "hide"]).unwrap();
    run(&["-"]).expect_err("用 mv 修改標籤失敗？");
}

const TALKER: &'static str = "--腳本小子";
const APPEND: &'static str = "第二行";
#[test]
fn test_args() {
    let _g = setup();
    run(&[
        "e",
        "test-with-args",
        "fast",
        &format!("echo -e \"$1：{}\n$2\"", MSG),
    ])
    .unwrap();
    assert_eq!(
        format!("{}：{}\n{}", TALKER, MSG, APPEND),
        run(&["-", TALKER, APPEND]).unwrap()
    );
}

#[test]
fn test_exact() {
    let _g = setup();
    run(&["e", "test-exact", "fast", "echo 'test exact!'"]).unwrap();
    run(&["tesct"]).expect("模糊搜不到東西！");
    run(&["=tesct"]).expect_err("打錯名字卻還搜得到！");
    run(&["=test-exact"]).expect("打完整名字卻搜不到！");
}

#[test]
fn test_prev() {
    let _g = setup();
    run(&["e", "test-prev1", "fast", "echo 'test prev 1'"]).unwrap();
    run(&["e", "test-prev2", "fast", "echo 'test prev 2'"]).unwrap();
    run(&["e", "test-prev3", "fast", "echo 'test prev 3'"]).unwrap();

    assert_eq!(run(&["^2"]).unwrap(), "test prev 2");
    assert_eq!(run(&["^2"]).unwrap(), "test prev 3");
    assert_eq!(run(&["^^^"]).unwrap(), "test prev 1");
    assert_eq!(run(&["cat", "^2"]).unwrap(), "echo 'test prev 3'");
    assert_eq!(
        run(&["-"]).unwrap(),
        "test prev 3",
        "cat 沒有確實影響到腳本時序"
    );

    run(&["^^^^"]).expect_err("明明只有三個腳本，跟我說有第四新的？");
}

#[test]
fn test_edit_same_name() {
    let _g = setup();
    run(&[
        "-t",
        "hide",
        "e",
        "i-am-hidden",
        "fast",
        &format!("echo \"{}\"", MSG),
    ])
    .unwrap();
    run(&["-"]).expect_err("執行了隱藏的腳本？？");
    run(&["e", "i-am-hidden", "fast", "yo"]).expect_err("竟然能編輯撞名的腳本？");
    run(&["tags", "hide"]).unwrap();
    assert_eq!(MSG, run(&["-"]).unwrap(), "腳本被撞名的編輯搞爛了？");
}
