#[allow(dead_code)]
#[path = "tool.rs"]
mod tool;

use hyper_scripter::util::read_file;
use std::fs::{create_dir_all, remove_dir_all, remove_file, File};
use std::io::prelude::*;
use tool::*;

fn test_import() {
    let tmp_dir = std::env::temp_dir();
    let dir_path = tmp_dir.join("to_be_import");
    let dir = dir_path.to_string_lossy();
    log::info!("把待匯入的腳本放進 {}", dir);

    run!("e copy/test -t +innate | echo 我要留下來").unwrap();
    run!(
        "e my/innate -t +innate | rm {} -rf && cp {}/tests/to_be_import {} -r",
        dir,
        env!("CARGO_MANIFEST_DIR"),
        dir,
    )
    .unwrap();

    run!("-f my -").unwrap();
    assert_eq!(run!("-f copy -").unwrap(), "我要留下來");

    run!(home: &dir_path, "--no-alias ls -la").expect_err("還沒升級就成功？");
    run!(home: &dir_path, "migrate").unwrap(); // NOTE: 順便測試 migrate 功能
    run!(home: &dir_path, "--no-alias ls -la").expect("升級了還失敗？");

    run!("tags something-evil").unwrap();
    run!("-f util import {}", dir).unwrap();
    run!("-f innate which myinnate").unwrap();

    assert_eq!(run!("-f my test").unwrap(), "安安！紅寶石");
    assert_eq!(run!("-f tag mytest").unwrap(), "安安！紅寶石");
    assert_eq!(run!("-f tag youtest").unwrap(), "殼已破碎");
    assert_eq!(run!("-f nameless -").unwrap(), "安安，匿名殼");
    assert_eq!(
        run!("-f copy -").unwrap(),
        "我要留下來",
        "匯入的腳本覆蓋掉舊腳本了"
    );

    run!("-f something-evil which -").expect_err("標籤匯入錯了？");
    run!("tags +all").unwrap();

    assert!(check_exist(&[".gitignore"]));

    assert_ls_len(16, Some("all"), None);

    run!("-f util import --namespace imported {}", dir).unwrap();
    // NOTE: 上面這行會噴一些找不到路徑的錯誤，不用緊張，是因為 `to_be_import` 裡面有些腳本被故意砍掉了
    assert_eq!(run!("-a imported/my/tes").unwrap(), "安安！紅寶石");
    run!("-f imported which").expect_err("命名空間汙染了標籤！");
    assert_ls_len(22, Some("all"), None);

    // check content of file
    let file_path = run!("which -a imported/my/tes").unwrap();
    let tmp_file_path = run!(home: &dir_path, "--no-alias which my/tes!").unwrap();
    assert_eq!(
        read_file(file_path.as_ref()).unwrap(),
        read_file(tmp_file_path.as_ref()).unwrap(),
        "匯入前後檔案內容不同"
    );
}

fn test_collect() {
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

    let named = create_all(
        "this/is/a/collect/t.est",
        Some("rb"),
        "puts '這是一個收集測試'",
    );
    let named_txt = create_all(
        "this/is/a/txt/coll.ect/test.ggext",
        None,
        "這是一個文字檔收集測試",
    );
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
    run!("-f innate ls myinnate").expect("還沒跑 collect 就壞掉了？");
    run!("-f my ls mytest").expect("還沒跑 collect 就壞掉了？");
    run!("-f all {}", named).expect_err("還沒收集就出現了，嚇死");

    run!("collect").unwrap();

    assert_eq!(run!("-f this {}", named).unwrap(), "這是一個收集測試");
    assert_eq!(run!("-f is {}", named).unwrap(), "這是一個收集測試");
    assert_eq!(run!("{}", named_txt).unwrap(), "這是一個文字檔收集測試");
    assert_eq!(run!(".10").unwrap(), "這是一個匿名收集測試");
    assert_eq!(run!(".100").unwrap(), "這是一個匿名文字檔收集測試");

    run!("-f all ls myinnate").expect_err("跑了 collect 沒有刪成功");
    run!("-f all ls =my/test").expect_err("跑了 collect 沒有刪成功"); // 需要 exact 因為還有另一個 imported/my/test
    run!("-f all ls =util/git").expect_err("跑了 collect 沒有刪成功"); // 同上
    run!("-f all ls .3").expect_err("跑了 collect 沒有刪成功");

    assert_eq!(run!("-f tag youest").unwrap(), "殼已破碎");
    assert_eq!(run!("-f nameless -").unwrap(), "安安，匿名殼");

    assert_ls_len(23, Some("all"), None);
}

#[test]
fn test_utils() {
    let _g = setup_with_utils();
    test_import();
    test_collect();
}
