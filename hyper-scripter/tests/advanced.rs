#[allow(dead_code)]
#[path = "tool.rs"]
mod tool;
pub use tool::*;

use hyper_scripter::{
    script_type::ScriptFullType,
    util::{read_file, remove, write_file},
};
use std::fs::remove_file;
use std::path::PathBuf;

#[test]
fn test_mv_dir() {
    let _g = setup();
    ScriptTest::new("file1", Some("removed"), None);
    ScriptTest::new("dir1/file1", None, None);
    ScriptTest::new("dir1/dir2/file1", None, None);
    ScriptTest::new("dir3/native", None, None);
    ScriptTest::new("dirless", None, None);

    run!("mv dir1/* dir3/ -t hide").expect_err("做為移動目標的兩個 file1 撞名");

    run!("mv d1d2 dir1/dir2/file2").unwrap();
    run!("mv dir1/* dir3/ -t hide").expect("不再撞名");
    assert_ls(vec!["dir3/native", "dirless"], None, None);
    run!("tags all,^removed").unwrap();
    assert_ls(
        vec!["dir3/file1", "dir3/file2", "dir3/native", "dirless"],
        None,
        None,
    );

    run!("mv dir3/file* /").expect_err("移動目標與既存的 file1 撞名");

    run!("mv file1! file4").unwrap();
    run!("mv dir3/file* /").expect("不再撞名");
    assert_ls(
        vec!["file1", "file2", "file4", "dir3/native", "dirless"],
        Some("all"),
        None,
    );
}

// TODO: test cp

#[test]
fn test_sub_tmpl() {
    let _g = setup();
    fn get_types_vec() -> Vec<String> {
        let mut v: Vec<_> = run!("types ls")
            .unwrap()
            .split_ascii_whitespace()
            .map(|s| s.to_owned())
            .collect();
        v.sort();
        v
    }
    fn modify_types_vec(og: &mut Vec<String>, new: &[&str], rmed: &[&str]) {
        og.retain(|s| !rmed.contains(&s.as_ref()));
        og.extend(new.iter().map(|s| s.to_string()));
        og.sort();
    }
    fn get_tmpl_path(name: &str) -> PathBuf {
        let ty: ScriptFullType = name.parse().unwrap();
        hyper_scripter::path::get_template_path(&ty).unwrap()
    }

    const RB_TRAVERSE: &str = "rb/traverse";
    const JS_SUPER_WIERD_NAME: &str = "js/suPeR-wiERd_NAme";
    const WIERD_JS_STR: &str = "this is a super wierd JS string";

    run!("e traverse-test -T {} | puts 'test!'", RB_TRAVERSE).unwrap();

    let mut types = get_types_vec();
    let p = get_tmpl_path(RB_TRAVERSE);
    remove(&p).unwrap();
    run!("e ? -T {} | puts 'test!'", RB_TRAVERSE).expect_err("子模版被砍了就不該創新的");

    assert_ne!(get_types_vec(), types); // rb/traverse 還在向量中
    modify_types_vec(&mut types, &[], &[RB_TRAVERSE]);
    assert_eq!(get_types_vec(), types);

    assert_eq!(
        run!("types js").unwrap(),
        run!("types {}", JS_SUPER_WIERD_NAME).unwrap(),
        "子模版的預設值應該和父類別相同（除非是寫死的那幾個，如 rb/traverse）"
    );
    let p = get_tmpl_path(JS_SUPER_WIERD_NAME);
    write_file(&p, &format!("console.log('{WIERD_JS_STR}')")).unwrap();
    assert_ne!(
        run!("types js").unwrap(),
        run!("types {}", JS_SUPER_WIERD_NAME).unwrap(),
        "子模版檔案已被寫入，不該相同"
    );

    assert_ne!(get_types_vec(), types); // 怪名字還不在向量中
    modify_types_vec(&mut types, &[JS_SUPER_WIERD_NAME], &[]);
    assert_eq!(get_types_vec(), types);

    run!("e wierd-test -T {} | dummy", JS_SUPER_WIERD_NAME).unwrap();
    assert_eq!(WIERD_JS_STR, run!("wierd-test").unwrap());
    run!("traverse-test").expect("刪個子模版不應影響已存在的腳本！");
}
