#[allow(dead_code)]
#[path = "tool.rs"]
mod tool;

use hyper_scripter::util::read_file;
use std::fs::{create_dir_all, remove_dir_all, remove_file, File};
use std::io::prelude::*;
use tool::*;

fn test_import(og_util_cnt: usize) {
    let tmp_dir = std::env::temp_dir();
    let dir_path = tmp_dir.join("to_be_import");
    let dir = dir_path.to_string_lossy();
    log::info!("把待匯入的腳本放進 {}", dir);

    run!("e copy/test -t +innate,copy | echo 我要留下來").unwrap();
    run!(
        "e my/innate -t +innate,my | rm {} -rf && cp {}/tests/to_be_import {} -r",
        dir,
        env!("CARGO_MANIFEST_DIR"),
        dir,
    )
    .unwrap();

    run!("-s my -").unwrap();
    assert_eq!(run!("-s copy -").unwrap(), "我要留下來");

    run!(allow_other_error: true, home: &dir_path, "--no-alias ls -la")
        .expect_err("還沒升級就成功？");
    run!(home: &dir_path, "migrate").unwrap(); // NOTE: 順便測試 migrate 功能
    run!(home: &dir_path, "--no-alias ls -la").expect("升級了還失敗？");

    run!("tags something-evil").unwrap();
    run!("-s util import {}", dir).unwrap();
    run!("-s innate which myinnate").unwrap();

    assert_eq!(run!("-s my test").unwrap(), "安安！紅寶石");
    assert_eq!(run!("-s tag mytest").unwrap(), "安安！紅寶石");
    assert_eq!(run!("-s tag youtest").unwrap(), "殼已破碎");
    assert_eq!(run!("-s nameless -").unwrap(), "安安，匿名殼");
    assert_eq!(
        run!("-s copy -").unwrap(),
        "我要留下來",
        "匯入的腳本覆蓋掉舊腳本了"
    );

    run!("-s something-evil which -").expect_err("標籤匯入錯了？");
    run!("tags +all").unwrap();

    assert!(check_exist(&[".gitignore"]));

    assert_ls_len(11 + og_util_cnt, Some("all"), None);

    run!("-s util import --namespace imported {}", dir).unwrap();
    // NOTE: 上面這行會噴一些找不到路徑的錯誤，不用緊張，是因為 `to_be_import` 裡面有些腳本被故意砍掉了
    assert_eq!(run!("-a imported/my/tes").unwrap(), "安安！紅寶石");
    run!("-s imported which").expect_err("命名空間汙染了標籤！");
    assert_ls_len(17 + og_util_cnt, Some("all"), None);

    // check content of file
    let file_path = run!("which -a imported/my/tes").unwrap();
    let tmp_file_path = run!(home: &dir_path, "--no-alias which my/tes!").unwrap();
    assert_eq!(
        read_file(file_path.as_ref()).unwrap(),
        read_file(tmp_file_path.as_ref()).unwrap(),
        "匯入前後檔案內容不同"
    );
}

fn test_collect(og_util_cnt: usize) {
    pub fn create_all(name: &str, ext: Option<&str>, content: &str) -> String {
        let full_name = if let Some(ext) = ext {
            format!("{}.{}", name, ext)
        } else {
            name.to_owned()
        };
        let p = get_home().join(full_name);

        if let Some(parent) = p.parent() {
            create_dir_all(parent).unwrap();
        }
        let mut file = File::create(p).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        format!("={}", name)
    }

    run!("e noughty-txt.sh -T txt | echo 別收集我").unwrap();

    const COLLECT_TXT: &str = "這是一個收集測試";
    let named = create_all(
        "this/is/a/collect/t.est",
        Some("rb"),
        &format!("puts '{COLLECT_TXT}'"),
    );
    const SCREWED_UP_TXT: &str = "因為副檔名爛掉所以被當作文字檔";
    let named_txt = create_all("this/is/a/txt/coll.ect/test.ggext", None, SCREWED_UP_TXT);
    create_all(".anonymous/10", Some("sh"), "echo 這是一個匿名收集測試");
    create_all(".anonymous/100", None, "這是一個匿名文字檔收集測試");

    create_all(
        "this/is/a/collect/.test.rb",
        None,
        "puts '這是一個不會被收集到的測試，因為路徑中帶.'",
    );

    create_all(
        "this/is/a/.collect/test.sh",
        None,
        "echo '這是一個不會被收集到的測試，因為路徑中帶.'",
    );

    remove_file(get_home().join("util/git")).unwrap(); // 刪掉 txt 檔
    remove_file(get_home().join(".anonymous/3")).unwrap(); // 刪掉 txt 檔
    remove_dir_all(get_home().join("my")).unwrap(); // 刪掉 myinnate 和 mytest
    run!("-s innate ls myinnate").expect("還沒跑 collect 就壞掉了？");
    run!("-s my ls mytest").expect("還沒跑 collect 就壞掉了？");
    run!("-s all {}", named).expect_err("還沒收集就出現了，嚇死");

    run!("collect").unwrap();

    assert_eq!(run!("{}", named).unwrap(), COLLECT_TXT);
    assert_eq!(run!("{}", named_txt).unwrap(), SCREWED_UP_TXT);
    assert_eq!(run!(".10").unwrap(), "這是一個匿名收集測試");
    assert_eq!(run!(".100").unwrap(), "這是一個匿名文字檔收集測試");

    run!("-s all ls myinnate").expect_err("跑了 collect 沒有刪成功");
    run!("-s all ls =my/test").expect_err("跑了 collect 沒有刪成功"); // 需要 exact 因為還有另一個 imported/my/test
    run!("-s all ls =util/git").expect_err("跑了 collect 沒有刪成功"); // 同上
    run!("-s all ls .3").expect_err("跑了 collect 沒有刪成功");

    assert_eq!(run!("-s tag youest").unwrap(), "殼已破碎");
    assert_eq!(run!("-s nameless -").unwrap(), "安安，匿名殼");

    assert_ls_len(18 + og_util_cnt, Some("all"), None);
}

#[test]
fn test_utils() {
    let _g = setup_with_utils();
    let og_util_cnt = get_ls(Some("all"), None).len();
    assert_eq!(og_util_cnt, 8, "original # of utils had changed!");
    test_import(og_util_cnt);
    test_collect(og_util_cnt);
}
