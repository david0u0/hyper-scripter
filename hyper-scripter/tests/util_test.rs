#![feature(str_split_once)]

#[allow(dead_code)]
#[path = "tool.rs"]
mod tool;

use hyper_scripter_util::get_all;
use std::fs::{create_dir_all, remove_dir_all, File};
use std::io::prelude::*;
use std::path::Path;
use std::sync::MutexGuard;
use tool::*;

pub fn setup_util<'a>() -> MutexGuard<'a, ()> {
    let g = setup();
    let utils = get_all();
    for u in utils.into_iter() {
        log::info!("載入 {}", u.name);
        run(format!(
            "e -c {} {} --no-template | {}",
            u.category, u.name, u.content
        ))
        .unwrap();
    }
    g
}

fn test_import() {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    let dir = Path::new(&out_dir).join("to_be_import");
    let dir = dir.to_string_lossy();

    run("e copy/test -f +innate | echo 我要留下來").unwrap();
    run(format!(
        "e my/innate -f +innate | cp tests/to_be_import {} -r",
        dir
    ))
    .unwrap();
    run("-f my -").unwrap();
    assert_eq!(run("-f copy -").unwrap(), "我要留下來");

    run("tags something-evil").unwrap();
    run(format!("-f util import {}", dir)).unwrap();
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

    run(format!("-f util import --namespace imported {}", dir)).unwrap();
    // NOTE: 上面這行會噴一些找不到路徑的錯誤，不用緊張，是因為 `to_be_import` 裡面有些腳本被故意砍掉了
    assert_eq!(run("-a imported/my/tes").unwrap(), "安安，紅寶石");
    run("-f imported which").expect_err("命名空間汙染了標籤！");
    assert_ls_len(25);
}

fn test_collect() {
    create_dir_all(get_home().join("this/is/a/collect")).unwrap();
    create_dir_all(get_home().join("this/is/a/.collect")).unwrap();

    let mut file = File::create(get_home().join("this/is/a/collect/t.est.rb")).unwrap();
    file.write_all("puts '這是一個收集測試'".as_bytes())
        .unwrap();
    let mut file = File::create(get_home().join(".anonymous/10.sh")).unwrap();
    file.write_all("echo 這是一個匿名收集測試".as_bytes())
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
    assert_eq!(run(".10").unwrap(), "這是一個匿名收集測試");
    run("-f innate which myinnate").expect_err("跑了 collect 沒有刪成功");
    run("-f my which =my/test").expect_err("跑了 collect 沒有刪成功");

    assert_eq!(run("-f tag youest").unwrap(), "殼已破碎");
    assert_eq!(run("-f nameless -").unwrap(), "安安，匿名殼");

    assert_ls_len(25);
}

#[test]
fn test_utils() {
    let _g = setup_util();
    test_import();
    test_collect();
}
