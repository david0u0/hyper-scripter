use regex::Regex;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard};

lazy_static::lazy_static! {
    static ref LOCK: Mutex<()> = Mutex::new(());
}

const PATH: &str = "./.hyper_scripter";

fn setup<'a>() -> MutexGuard<'a, ()> {
    let guard = LOCK.lock().unwrap_or_else(|err| err.into_inner());
    let _ = env_logger::try_init();
    match std::fs::remove_dir_all(PATH) {
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
    let mut file: PathBuf = PATH.into();
    for p in p.iter() {
        file = file.join(p);
    }
    file.exists()
}
fn run(args: &[&str]) -> Result<String, i32> {
    let mut full_args = vec!["-p", PATH];
    full_args.extend(args);

    let mut cmd: Command;
    #[cfg(not(debug_assertions))]
    {
        cmd = Command::new("../target/release/hyper-scripter");
    }
    #[cfg(debug_assertions)]
    {
        cmd = Command::new("../target/debug/hyper-scripter");
    }
    let mut child = cmd.args(&full_args).stdout(Stdio::piped()).spawn().unwrap();
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
    run(&["e", ".", &format!("echo \"{}\"", MSG), "--fast"]).unwrap();
    assert_eq!(MSG, run(&["-"]).unwrap());

    run(&[
        "-f",
        "super_tag,hide",
        "e",
        "test/js",
        "-c",
        "js",
        "--fast",
        &format!("console.log(\"{}\")", MSG_JS),
    ])
    .unwrap();
    run(&["tesjs"]).expect_err("標籤沒有篩選掉不該出現的腳本！");
    assert_eq!(MSG_JS, run(&["-f", "super_tag", "-"]).unwrap());

    assert_eq!(MSG, run(&[".1"]).expect("標籤篩選把舊的腳本搞爛了！"));

    run(&["tesjs"]).expect_err("標籤沒有篩選掉不該出現的腳本！可能是上上個操作把設定檔寫爛了");
    run(&["tags", "all"]).unwrap();
    run(&["tags", "no-hidden=all"]).unwrap();
    run(&["tesjs"]).expect("沒吃到設定檔的標籤？");
    run(&["tags", "test"]).unwrap();
    run(&["tesjs"]).expect("命名空間沒賦與它標籤？");
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
        "--fast",
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
        "--fast",
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
    run(&["e", "test-exact", "--fast", "echo 'test exact!'"]).unwrap();
    run(&["tesct"]).expect("模糊搜不到東西！");
    run(&["=tesct"]).expect_err("打錯名字卻還搜得到！");
    run(&["=test-exact"]).expect("打完整名字卻搜不到！");
}

#[test]
fn test_prev() {
    let _g = setup();

    run(&["e", "test-prev1", "--fast", "echo 'test prev 1'"]).unwrap();
    run(&["e", "test-prev2", "--fast", "echo 'test prev 2'"]).unwrap();
    run(&["e", "test-prev3", "-n", "--fast", "echo 'test prev 3'"]).unwrap();

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
        "-f",
        "hide",
        "e",
        "i-am-hidden",
        "--fast",
        &format!("echo \"{}\"", MSG),
    ])
    .unwrap();
    run(&["-"]).expect_err("執行了隱藏的腳本？？");
    run(&["e", "i-am-hidden", "--fast", "yo"]).expect_err("竟然能編輯撞名的腳本？");
    assert_eq!(
        MSG,
        run(&["-f", "hide", "-"]).unwrap(),
        "腳本被撞名的編輯搞爛了？"
    );
}

#[test]
fn test_multi_filter() {
    let _g = setup();
    run(&["e", "nobody", "--fast", &format!("echo \"{}\"", MSG)]).unwrap();
    run(&[
        "-f",
        "test,pin",
        "e",
        "test-pin",
        "--fast",
        &format!("echo \"{}\"", MSG),
    ])
    .unwrap();
    run(&[
        "-f",
        "pin",
        "e",
        "pin-only",
        "--fast",
        &format!("echo \"{}\"", MSG),
    ])
    .unwrap();

    assert_eq!(MSG, run(&["pin-only"]).unwrap());
    assert_eq!(MSG, run(&["test-pin"]).unwrap());
    assert_eq!(MSG, run(&["nobody"]).unwrap());

    run(&["tags", "+hidden"]).unwrap();
    assert_eq!(MSG, run(&["pin-only"]).unwrap());
    assert_eq!(MSG, run(&["test-pin"]).unwrap());
    run(&["nobody"]).expect_err("未能被主篩選器篩掉");

    run(&["tags", "+^test"]).unwrap();
    assert_eq!(MSG, run(&["pin-only"]).unwrap());
    run(&["test-pin"]).expect_err("未能被主篩選器篩掉");

    assert_eq!(MSG, run(&["-a", "test-pin"]).unwrap());
}

#[test]
fn test_rm() {
    let _g = setup();
    run(&["e", "longlive", "--fast", "echo 矻立不搖"]).unwrap();

    run(&["e", "test", "--fast", &format!("echo \"{}\"", MSG)]).unwrap();
    assert_eq!(MSG, run(&["test"]).unwrap());
    run(&["rm", "-"]).unwrap();
    run(&["test"]).expect_err("未能被刪除掉");
    run(&["-a", "test"]).expect_err("被刪除掉的腳本竟能用 `-a` 找回來");
    assert_eq!(MSG, run(&["-f", "deleted", "test"]).unwrap());

    assert_eq!("矻立不搖", run(&["longlive"]).unwrap());

    run(&["e", ".", "--fast", "echo \"你匿\""]).unwrap();
    assert_eq!("你匿", run(&[".1"]).unwrap());
    run(&["rm", "-"]).unwrap();
    assert_eq!(
        "你匿",
        run(&["-f", "deleted", "-"]).expect("就算是匿名腳本也不該真的被刪掉！")
    );

    assert_eq!("矻立不搖", run(&["longlive"]).unwrap());

    run(&[
        "e",
        "my-namespace/super-test",
        "--fast",
        "echo \"不要刪我 QmmmmQ\"",
    ])
    .unwrap();
    assert_eq!("不要刪我 QmmmmQ", run(&["my-super-test"]).unwrap());
    run(&["rm", "mysupertest"]).expect("刪除被命名空間搞爛了");
    run(&["my-super-test"]).expect_err("未能被刪除掉");
    assert_eq!(
        "不要刪我 QmmmmQ",
        run(&["-f", "deleted", "my-namespace/super-test"]).unwrap()
    );
    let file_path = run(&["-f", "deleted", "which", "-"]).unwrap();
    let re = Regex::new(r".+my-namespace/\d{14}-super-test\.sh$").unwrap();
    assert!(re.is_match(&file_path), "路徑被刪除改爛：{}", file_path);

    assert_eq!("矻立不搖", run(&["longlive"]).unwrap());

    assert!(check_exist(&["longlive.sh"]));
    run(&["rm", "--purge", "*", "-f", "all"]).expect("未能消滅掉一切");
    run(&["-f", "all", "longlive"]).expect_err("沒有確實消滅掉");
    assert!(!check_exist(&["longlive.sh"]));

    run(&["-f", "all", "my-super-test"]).expect_err("沒有確實消滅掉");
}

#[test]
fn test_namespace_reorder_search() {
    let _g = setup();
    run(&[
        "e",
        "my/super/long/namespace-d/test-script",
        "--fast",
        &format!("echo \"{}\"", MSG),
    ])
    .unwrap();
    run(&[
        "e",
        "a/shorter/script",
        "-c",
        "js",
        "--fast",
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

#[test]
fn test_append_tags() {
    let _g = setup();
    run(&["tags", "global"]).unwrap();
    run(&[
        "-f",
        "+append",
        "e",
        "append-test",
        "--fast",
        &format!("echo 附加標籤"),
    ])
    .unwrap();
    run(&[
        "-f",
        "no-append",
        "e",
        "no-append-test",
        "--fast",
        &format!("echo 不要給我打標籤"),
    ])
    .unwrap();

    assert_eq!("附加標籤", run(&["apptest"]).unwrap());
    run(&["no-apptest"]).expect_err("標籤還是附加上去了？");

    assert_eq!(
        "附加標籤",
        run(&["-f", "append", "apptest"]).expect("標籤沒附加上去？")
    );
    assert_eq!(
        "不要給我打標籤",
        run(&["-f", "no-append", "apptest"]).unwrap()
    );

    run(&[
        "-f",
        "no-append",
        "mv",
        "no-append-test",
        "-t",
        "+eventually-append",
    ])
    .unwrap();
    assert_eq!(
        "不要給我打標籤",
        run(&["-f", "eventually-append", "apptest"]).expect("標籤沒被 mv 附加上去？")
    );
    assert_eq!(
        "不要給我打標籤",
        run(&["-f", "no-append", "apptest"]).expect("標籤被 mv 弄壞了？")
    );
}
