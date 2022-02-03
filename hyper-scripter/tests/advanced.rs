#[allow(dead_code)]
#[path = "tool.rs"]
mod tool;
pub use tool::*;

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
