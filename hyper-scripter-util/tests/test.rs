#[allow(dead_code)]
#[path = "../../hyper-scripter-test-lib/test_util.rs"]
mod test_util;

use hyper_scripter_util::get_all;
use std::fs::{create_dir_all, File};
use std::io::prelude::*;
use std::path::PathBuf;
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

fn test_import() {
    run(&[
        "e",
        "my/innate",
        "cp tests/to_be_import ./.tmp -r",
        "--fast",
        "-f",
        "+innate",
    ])
    .unwrap();
    run(&["-f", "my", "-"]).unwrap();

    run(&["tags", "something-evil"]).unwrap();
    run(&["-f", "util", "import", ".tmp"]).unwrap();
    run(&["-f", "innate", "which", "myinnate"]).unwrap();

    assert_eq!(run(&["-f", "my", "test"]).unwrap(), "安安，紅寶石");
    assert_eq!(run(&["-f", "tag", "mytest"]).unwrap(), "安安，紅寶石");
    assert_eq!(run(&["-f", "tag", "youtest"]).unwrap(), "安安，殼");
    assert_eq!(run(&["-f", "nameless", "-"]).unwrap(), "安安，匿名殼");

    run(&["-f", "something-evil", "which", "-"]).expect_err("標籤匯入錯了？");
    run(&["tags", "+all"]).unwrap();
}

const GITIGNORE_CONTENT: &'static str = ".script_history.db
*.db-*
.hs_exe_path
";
fn test_git() {
    run(&["-a", "git", "init"]).unwrap();
    assert_eq!(GITIGNORE_CONTENT, read(&[".gitignore"]));
}
fn test_collect() {
    let p: PathBuf = PATH.into();
    create_dir_all(p.join("this/is/a/collect")).unwrap();
    let mut file = File::create(p.join("this/is/a/collect/test.rb")).unwrap();
    file.write_all("puts '這是一個收集測試'".as_bytes())
        .unwrap();
    run(&["thisisacolltest"]).expect_err("還沒收集就出現了，嚇死");
    run(&["collect"]).unwrap();
    assert_eq!(
        run(&["-f", "this", "thisisacolltest"]).unwrap(),
        "這是一個收集測試"
    );
    assert_eq!(
        run(&["-f", "is", "thisisacolltest"]).unwrap(),
        "這是一個收集測試"
    );
}

#[test]
fn test_utils() {
    let _g = setup_util();
    test_import();
    run(&["tags", "all"]).unwrap();
    test_git();
    test_collect();
}
