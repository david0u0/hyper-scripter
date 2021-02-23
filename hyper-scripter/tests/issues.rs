#![feature(str_split_once)]

#[allow(dead_code)]
#[path = "tool.rs"]
mod tool;

use std::fs::remove_file;
use tool::*;

#[test]
fn test_mv_same_name() {
    println!("移動腳本時，若前後腳本的路徑相同（腳本分類所致），應順利改動");
    let _g = setup();

    run("e test | echo 1").unwrap();
    run("e test2 | echo 2").unwrap();

    run("mv test -c tmux").unwrap();
    assert_eq!(run("test").unwrap(), "1");

    run("mv test test2").expect_err("移動成撞名的腳本應報錯");
    assert_eq!(run("test").unwrap(), "1");
    run("mv test test2 -c rb").expect_err("移動成撞名的腳本，即使分類不同，也應報錯");
    assert_eq!(run("test").unwrap(), "1");
}
#[test]
fn test_cp_same_name() {
    println!("複製腳本時，若和既存的腳本撞名，應報錯");
    let _g = setup();

    run("e test -c rb | puts 1").unwrap();
    run("e test2 | echo 2").unwrap();
    run("e test3 -c rb | puts 3").unwrap();

    run("cp test test2").expect_err("改成撞名的腳本，即使路徑不同，也應報錯");
    assert_eq!(run("test2").unwrap(), "2");

    run("cp test test3").expect_err("改成撞名的腳本應報錯");
    assert_eq!(run("test3").unwrap(), "3");

    assert_eq!(run("test").unwrap(), "1");
}
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
#[test]
fn test_hs_in_hs() {
    println!("若腳本甲裡呼叫了本程式去執行腳本乙，完成之後腳本甲的時間應較新");
    let _g = setup();

    run(format!(
        "e outer | echo 我在第一層 && {} -H {} inner",
        get_exe_abs(),
        get_home().to_string_lossy(),
    ))
    .unwrap();
    run("e inner | echo '我在第幾層？'").unwrap();

    assert_eq!(run("-").unwrap(), "我在第幾層？");
    assert_eq!(run("outer").unwrap(), "我在第一層\n我在第幾層？");
    assert_eq!(run("-").unwrap(), "我在第一層\n我在第幾層？");
    assert_eq!(run("^2").unwrap(), "我在第幾層？");
}
#[tokio::test(threaded_scheduler)]
async fn test_edit_existing_bang() {
    println!("用 BANG! 編輯已存在的腳本，不該出錯");
    let _g = setup();

    run("e test -t hide | echo 躲貓貓").unwrap();

    use hyper_scripter::script_repo::ScriptRepo;
    use hyper_scripter::tag::{Tag, TagFilter};
    use hyper_scripter::util::main_util::{edit_or_create, EditTagArgs};

    let (pool, _) = hyper_scripter::db::get_pool().await.unwrap();
    let mut repo = ScriptRepo::new(pool, None).await.unwrap();
    repo.filter_by_tag(&"all,^hide".parse::<TagFilter>().unwrap().into());

    edit_or_create(
        "test".parse().unwrap(),
        &mut repo,
        None,
        EditTagArgs {
            content: "gg".parse().unwrap(),
            change_existing: true,
            append_namespace: true,
        },
    )
    .await
    .expect_err("沒有 BANG! 就找到編輯的腳本！？");

    let (p, e) = edit_or_create(
        "test!".parse().unwrap(),
        &mut repo,
        None,
        EditTagArgs {
            content: "+a,^b,c".parse().unwrap(),
            change_existing: true,
            append_namespace: true,
        },
    )
    .await
    .unwrap();

    assert_eq!(p, get_home().join("test.sh"));

    use fxhash::FxHashSet as HashSet;
    let mut tags = HashSet::<Tag>::default();
    tags.insert("a".parse().unwrap());
    tags.insert("c".parse().unwrap());
    tags.insert("hide".parse().unwrap());
    assert_eq!(tags, e.tags);
}
