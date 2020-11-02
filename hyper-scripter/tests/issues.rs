#![feature(str_split_once)]

#[allow(dead_code)]
#[path = "tool.rs"]
mod tool;

use std::fs::remove_file;
use tool::*;

#[test]
fn test_rm_non_exist() {
    println!("若欲刪除的腳本不存在，應直接消滅之。");

    let _g = setup();
    run("e test | echo '刪我啊'").unwrap();
    run("e test2 | echo '別刪我QQ'").unwrap();
    assert_eq!(run("test").unwrap(), "刪我啊");
    let file = get_home().join("test.sh");
    remove_file(&file).unwrap();
    assert_ls_len(2);
    assert_eq!(run("which test").unwrap(), file.to_string_lossy()); // TODO: 應該允許 which 嗎？

    run("test").expect_err("刪掉的腳本還能執行！？");
    run("rm *").expect("rm 應該要消滅掉不存在的腳本");

    assert_ls_len(1);
}
