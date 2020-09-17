use hyper_scripter::path;
use regex::Regex;
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
fn check_exist(p: &[&str]) -> bool {
    let mut file = path::get_path();
    for p in p.iter() {
        file = file.join(p);
    }
    file.exists()
}
fn run(args: &[&str]) -> Result<String, i32> {
    let mut cmd = Command::new("./target/debug/hyper_scripter");
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
const MSG_JS: &'static str = "你好，爪哇腳本人！";
#[test]
fn test_tags() {
    let _g = setup();
    run(&["e", ".", &format!("echo \"{}\"", MSG), "-f"]).unwrap();
    assert_eq!(MSG, run(&["-"]).unwrap());

    run(&[
        "-t",
        "super_tag,hide",
        "e",
        "test_js",
        "-c",
        "js",
        "-f",
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

    run(&[
        "e",
        ".",
        "-c",
        "js",
        "--no-template",
        "-f",
        &format!("echo \"{}\"", MSG),
    ])
    .unwrap();
    run(&["-"]).expect_err("用 nodejs 執行 echo ……？");

    run(&["mv", "1", "-c", "sh"]).unwrap();
    assert_eq!(MSG, run(&["-"]).unwrap());
    assert!(check_exist(&[".anonymous", "1.sh"]), "改腳本類型失敗");
    assert!(
        !check_exist(&[".anonymous", "1.js"]),
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
        "-f",
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
    run(&["e", "test-exact", "-f", "echo 'test exact!'"]).unwrap();
    run(&["tesct"]).expect("模糊搜不到東西！");
    run(&["=tesct"]).expect_err("打錯名字卻還搜得到！");
    run(&["=test-exact"]).expect("打完整名字卻搜不到！");
}

#[test]
fn test_prev() {
    let _g = setup();

    run(&["e", "test-prev1", "-f", "echo 'test prev 1'"]).unwrap();
    run(&["e", "test-prev2", "-f", "echo 'test prev 2'"]).unwrap();
    run(&["e", "test-prev3", "-n", "-f", "echo 'test prev 3'"]).unwrap();

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
        "-f",
        &format!("echo \"{}\"", MSG),
    ])
    .unwrap();
    run(&["-"]).expect_err("執行了隱藏的腳本？？");
    run(&["e", "i-am-hidden", "-f", "yo"]).expect_err("竟然能編輯撞名的腳本？");
    run(&["tags", "hide"]).unwrap();
    assert_eq!(MSG, run(&["-"]).unwrap(), "腳本被撞名的編輯搞爛了？");
}

#[test]
fn test_multi_filter() {
    let _g = setup();
    run(&["e", "nobody", "-f", &format!("echo \"{}\"", MSG)]).unwrap();
    run(&[
        "-t",
        "test,pin",
        "e",
        "test-pin",
        "-f",
        &format!("echo \"{}\"", MSG),
    ])
    .unwrap();
    run(&[
        "-t",
        "pin",
        "e",
        "pin-only",
        "-f",
        &format!("echo \"{}\"", MSG),
    ])
    .unwrap();

    assert_eq!(MSG, run(&["pin-only"]).unwrap());
    assert_eq!(MSG, run(&["test-pin"]).unwrap());
    assert_eq!(MSG, run(&["nobody"]).unwrap());

    run(&["tags", "hidden"]).unwrap();
    assert_eq!(MSG, run(&["pin-only"]).unwrap());
    assert_eq!(MSG, run(&["test-pin"]).unwrap());
    run(&["nobody"]).expect_err("未能被主篩選器篩掉");

    run(&["tags", "^test"]).unwrap();
    assert_eq!(MSG, run(&["pin-only"]).unwrap());
    run(&["test-pin"]).expect_err("未能被主篩選器篩掉");

    assert_eq!(MSG, run(&["-a", "test-pin"]).unwrap());
}

#[test]
fn test_rm() {
    let _g = setup();
    run(&["e", "longlive", "-f", "echo 矻立不搖"]).unwrap();

    run(&["e", "test", "-f", &format!("echo \"{}\"", MSG)]).unwrap();
    assert_eq!(MSG, run(&["test"]).unwrap());
    run(&["rm", "-"]).unwrap();
    run(&["test"]).expect_err("未能被刪除掉");
    run(&["-a", "test"]).expect_err("被刪除掉的腳本竟能用 `-a` 找回來");
    assert_eq!(MSG, run(&["-t", "deleted", "test"]).unwrap());

    assert_eq!("矻立不搖", run(&["longlive"]).unwrap());

    run(&["e", ".", "-f", "echo \"你匿\""]).unwrap();
    assert_eq!("你匿", run(&["-"]).unwrap());
    run(&["rm", "-"]).unwrap();
    run(&["-t", "deleted", ".1"]).expect_err("被刪掉的匿名腳本還能找得回來");

    assert_eq!("矻立不搖", run(&["longlive"]).unwrap());

    run(&[
        "e",
        "my-namespace/super-test",
        "-f",
        "echo \"不要刪我 QmmmmQ\"",
    ])
    .unwrap();
    assert_eq!("不要刪我 QmmmmQ", run(&["my-super-test"]).unwrap());
    run(&["rm", "my-super-test"]).expect("刪除被命名空間搞爛了");
    run(&["my-super-test"]).expect_err("未能被刪除掉");
    assert_eq!(
        "不要刪我 QmmmmQ",
        run(&["-t", "deleted", "my-namespace/super-test"]).unwrap()
    );
    let file_path = run(&["-t", "deleted", "which", "-"]).unwrap();
    let re = Regex::new(r"^my-namespace/\d{14}-super-test\.sh$").unwrap();
    assert!(re.is_match(&file_path), "路徑被刪除改爛：{}", file_path);

    assert_eq!("矻立不搖", run(&["longlive"]).unwrap());
}

#[test]
fn test_namespace_reorder_search() {
    let _g = setup();
    run(&[
        "e",
        "my/super/long/namespace-d/test-script",
        "-f",
        &format!("echo \"{}\"", MSG),
    ])
    .unwrap();
    run(&[
        "e",
        "a/shorter/script",
        "-c",
        "js",
        "-f",
        &format!("console.log(\"{}\")", MSG_JS),
    ])
    .unwrap();
    assert_eq!(MSG, run(&["myscript"]).expect("正常空間搜尋失敗"));
    assert_eq!(MSG, run(&["scriptsuper"]).expect("重排命名空間搜尋失敗"));
    assert_eq!(MSG, run(&["testlong"]).expect("重排命名空間搜尋失敗"));
    assert_eq!(MSG_JS, run(&["scrishorter"]).expect("重排命名空間搜尋失敗"));
    assert_eq!(MSG, run(&["namsplongsuery"]).expect("重排命名空間搜尋失敗"));
    run(&["script-test"]).expect_err("重排到腳本名字去了= =");
}
