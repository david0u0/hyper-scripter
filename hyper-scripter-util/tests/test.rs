#[allow(dead_code)]
#[path = "../../hyper-scripter-test-lib/test_util.rs"]
mod test_util;

use hyper_scripter_util::get_all;
use std::sync::MutexGuard;
use test_util::*;

pub fn setup_util<'a>() -> MutexGuard<'a, ()> {
    let g = setup();
    if !check_exist(&["."]) {
        let utils = get_all();
        for u in utils.into_iter() {
            run(&[
                "e",
                "-c",
                u.category,
                u.name,
                u.content,
                "--fast",
                "--no-template",
            ])
            .unwrap();
        }
    }
    g
}
fn test_banish() {
    run(&["tags", "+test"]).unwrap();

    run(&["e", "my/test1", "echo my/test1!", "--fast"]).unwrap();
    run(&["e", "my/test2", "echo my/test2!", "--fast"]).unwrap();

    run(&["rm", "-"]).unwrap();
    run(&["rm", "-"]).unwrap();
    assert_eq!(run(&["-f", "all", "mytest1"]).unwrap(), "my/test1!");
    assert_eq!(run(&["-f", "all", "mytest2"]).unwrap(), "my/test2!");

    run(&["banish"]).unwrap();
    run(&["-f", "all", "mytest1"]).expect_err("沒有放逐成功 QQ");
    run(&["-f", "all", "mytest2"]).expect_err("沒有放逐成功 QQ");
}

fn test_import() {
    run(&[
        "e",
        "my/innate",
        "-c",
        "js",
        "console.log('安安，爪哇腳本')",
        "--fast",
        "-f",
        "+innate",
    ])
    .unwrap();
    run_with_home(
        ".tmp",
        &[
            "e",
            "my/test",
            "-c",
            "rb",
            "puts '安安，紅寶石'",
            "--fast",
            "-f",
            "+tag",
        ],
    )
    .unwrap();
    run_with_home(
        ".tmp",
        &["e", "your/test", "echo '安安，殼'", "--fast", "-f", "+tag"],
    )
    .unwrap();
    run_with_home(".tmp", &["rm", "youtest"]).unwrap();

    run(&["tags", "something-evil"]).unwrap();
    run(&["-f", "util", "import", ".tmp"]).unwrap();
    assert_eq!(run(&["-f", "my", "myinnate"]).unwrap(), "安安，爪哇腳本");
    assert_eq!(
        run(&["-f", "innate", "myinnate"]).unwrap(),
        "安安，爪哇腳本"
    );
    assert_eq!(run(&["-f", "my", "test"]).unwrap(), "安安，紅寶石");
    assert_eq!(run(&["-f", "tag", "mytest"]).unwrap(), "安安，紅寶石");
    assert_eq!(run(&["-f", "tag", "youtest"]).unwrap(), "安安，殼");

    run(&["-f", "something-evil", "-"]).expect_err("標籤匯入錯了？");
}

const GITIGNORE_CONTENT: &'static str = ".script_history.db
*.db-*
.hs_exe_path
";
fn test_git() {
    run(&["git", "init"]).unwrap();
    assert_eq!(GITIGNORE_CONTENT, read(&[".gitignore"]));
}

#[test]
fn test_utils() {
    let _g = setup_util();
    test_import();
    test_banish();
    test_git();
}
