#![feature(str_split_once)]

#[allow(dead_code)]
#[path = "../../hyper-scripter-test-lib/test_util.rs"]
mod test_util;

use hyper_scripter_util::get_all;
use std::fs::{create_dir_all, remove_dir_all, File};
use std::io::prelude::*;
use std::sync::MutexGuard;
use test_util::*;

pub fn setup_util<'a>() -> MutexGuard<'a, ()> {
    let g = setup();
    let utils = get_all();
    for u in utils.into_iter() {
        log::info!("載入 {}", u.name);
        run(&format!(
            "e -c {} {} --no-template | {}",
            u.category, u.name, u.content
        ))
        .unwrap();
    }
    g
}

fn assert_ls_len(expect: usize) {
    let ls_res = run("ls -f all --grouping none --plain --name").unwrap();
    let ls_vec = ls_res
        .split(" ")
        .filter(|s| s.len() > 0)
        .collect::<Vec<_>>();
    assert_eq!(expect, ls_vec.len(), "ls 結果為 {:?}", ls_vec);
}

fn test_import() {
    run("e copy/test -f +innate | echo 我要留下來").unwrap();
    run("e my/innate -f +innate | cp tests/to_be_import ./.tmp -r").unwrap();
    run("-f my -").unwrap();
    assert_eq!(run("-f copy -").unwrap(), "我要留下來");

    run("tags something-evil").unwrap();
    run("-f util import .tmp").unwrap();
    run("-f innate which myinnate").unwrap();

    assert_eq!(run("-f my test").unwrap(), "安安，紅寶石");
    assert_eq!(run("-f tag mytest").unwrap(), "安安，紅寶石");
    assert_eq!(run("-f tag youtest").unwrap(), "殼已破碎");
    assert_eq!(run("-f nameless -").unwrap(), "安安，匿名殼");
    assert_eq!(run("-f copy -").unwrap(), "我要留下來");

    run("-f something-evil which -").expect_err("標籤匯入錯了？");
    run("tags +all").unwrap();

    assert!(check_exist(&[".gitignore"]));

    assert_ls_len(16);
}

fn test_collect() {
    create_dir_all(get_home().join("this/is/a/collect")).unwrap();
    create_dir_all(get_home().join("this/is/a/.collect")).unwrap();

    let mut file = File::create(get_home().join("this/is/a/collect/t.est.rb")).unwrap();
    file.write_all("puts '這是一個收集測試'".as_bytes())
        .unwrap();

    let mut file = File::create(get_home().join("this/is/a/collect/.test.rb")).unwrap();
    file.write_all("puts '這是一個不會被收集到的測試，因為路徑中帶.'".as_bytes())
        .unwrap();

    let mut file = File::create(get_home().join("this/is/a/.collect/test.sh")).unwrap();
    file.write_all("echo '這是一個不會被收集到的測試，因為路徑中帶.'".as_bytes())
        .unwrap();

    remove_dir_all(get_home().join("my")).unwrap(); // 刪掉 myinnate 和 mytest
    run("-f innate which myinnate").expect("還沒跑 collect 就壞掉了？");
    run("-f my which mytest").expect("還沒跑 collect 就壞掉了？");
    run("thisisacolltest").expect_err("還沒收集就出現了，嚇死");

    run("collect").unwrap();
    assert_eq!(run("-f this thisisacolltest").unwrap(), "這是一個收集測試");
    assert_eq!(run("-f is thisisacolltest").unwrap(), "這是一個收集測試");
    run("-f innate which myinnate").expect_err("跑了 collect 沒有刪成功");
    run("-f my which mytest").expect_err("跑了 collect 沒有刪成功");

    assert_eq!(run("-f tag youest").unwrap(), "殼已破碎");
    assert_eq!(run("-f nameless -").unwrap(), "安安，匿名殼");

    assert_ls_len(15);
}

#[test]
fn test_utils() {
    let _g = setup_util();
    test_import();
    test_collect();
}
