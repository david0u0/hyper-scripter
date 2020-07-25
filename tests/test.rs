use instant_scripter::path;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn setup(name: &str) -> Env {
    let _ = env_logger::try_init();
    let p = path::get_sys_path().unwrap().join(format!("test_{}", name));
    match std::fs::remove_dir_all(&p) {
        Ok(_) => (),
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                panic!("重整測試用資料夾失敗了……")
            }
        }
    }
    Env(p)
}
struct Env(PathBuf);
impl Env {
    pub fn check_exist(&self, path: &str) -> bool {
        self.0.join(path).exists()
    }
    pub fn run(&self, args: &[&str]) -> Result<String, i32> {
        let mut cmd = Command::new("./target/debug/instant_scripter");
        let mut full_args = vec!["-p", self.0.to_str().unwrap()];
        full_args.extend(args);
        let mut child = cmd.args(full_args).stdout(Stdio::piped()).spawn().unwrap();
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
}

const MSG: &'static str = "你好，腳本管理員！";
const MSG_JS: &'static str = "你好，腳本管理員！.js";
#[test]
fn test_tags() {
    let r = setup("tags");
    r.run(&["e", "-c", &format!("echo \"{}\"", MSG)]).unwrap();
    assert_eq!(MSG, r.run(&["-"]).unwrap());

    r.run(&[
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
    r.run(&["tesjs"])
        .expect_err("標籤沒有篩選掉不該出現的腳本！");
    assert_eq!(MSG_JS, r.run(&["-t", "super_tag", "-"]).unwrap());

    assert_eq!(MSG, r.run(&[".1"]).expect("標籤篩選把舊的腳本搞爛了！"));

    r.run(&["tesjs"])
        .expect_err("標籤沒有篩選掉不該出現的腳本！可能是上上個操作把設定檔寫爛了");
    r.run(&["tags", "all"]).unwrap();
    r.run(&["tesjs"]).expect("沒吃到設定檔的標籤？");
}

#[test]
fn test_mv() {
    let r = setup("mv");
    r.run(&["e", "-x", "js", "-c", &format!("echo \"{}\"", MSG)])
        .unwrap();
    r.run(&["-"]).expect_err("用 nodejs 執行 echo ……？");

    r.run(&["mv", "1", "-x", "sh"]).unwrap();
    assert_eq!(MSG, r.run(&["-"]).unwrap());
    assert!(r.check_exist(".anonymous/1.sh"), "改腳本類型失敗");
    assert!(
        !r.check_exist(".anonymous/1.js"),
        "改了腳本類型舊檔案還留著？"
    );

    r.run(&["mv", "1", "-t", "hide"]).unwrap();
    r.run(&["-"]).expect_err("用 mv 修改標籤失敗？");
}

const TALKER: &'static str = "--腳本小子";
const APPEND: &'static str = "第二行";
#[test]
fn test_args() {
    let r = setup("args");
    r.run(&[
        "e",
        "test-with-args",
        "-c",
        &format!("echo -e \"$1：{}\n$2\"", MSG),
    ])
    .unwrap();
    assert_eq!(
        format!("{}：{}\n{}", TALKER, MSG, APPEND),
        r.run(&["-", TALKER, APPEND]).unwrap()
    );
}

#[test]
fn test_exact() {
    let r = setup("exact");
    r.run(&["e", "test-exact", "-c", "echo 'test exact!'"])
        .unwrap();
    r.run(&["tesct"]).expect("模糊搜不到東西！");
    r.run(&["=tesct"]).expect_err("打錯名字卻還搜得到！");
    r.run(&["=test-exact"]).expect("打完整名字卻搜不到！");
}

#[test]
fn test_prev() {
    let r = setup("prev");
    r.run(&["e", "test-prev1", "-c", "echo 'test prev 1'"])
        .unwrap();
    r.run(&["e", "test-prev2", "-c", "echo 'test prev 2'"])
        .unwrap();
    r.run(&["e", "test-prev3", "-c", "echo 'test prev 3'"])
        .unwrap();

    assert_eq!(r.run(&["^2"]).unwrap(), "test prev 2");
    assert_eq!(r.run(&["^2"]).unwrap(), "test prev 3");
    assert_eq!(r.run(&["^^^"]).unwrap(), "test prev 1");
    assert_eq!(r.run(&["cat", "^2"]).unwrap(), "echo 'test prev 3'");
    assert_eq!(
        r.run(&["-"]).unwrap(),
        "test prev 3",
        "cat 沒有確實影響到腳本時序"
    );

    r.run(&["^^^^"])
        .expect_err("明明只有三個腳本，跟我說有第四新的？");
}
