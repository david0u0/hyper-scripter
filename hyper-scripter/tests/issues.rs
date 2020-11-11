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
#[tokio::test(threaded_scheduler)]
async fn test_edit_existing_bang() {
    println!("用 BANG! 編輯已存在的腳本，不該出錯");
    let _g = setup();

    run("e test -t hide | echo 躲貓貓").unwrap();

    use hyper_scripter::script_repo::ScriptRepo;
    use hyper_scripter::tag::{Tag, TagFilter};
    use hyper_scripter::util::main_util::{edit_or_create, EditTagArgs};
    use std::str::FromStr;

    let pool = hyper_scripter::db::get_pool().await.unwrap();
    let mut repo = ScriptRepo::new(pool, None).await.unwrap();
    let main_filter = TagFilter::from_str("all,^hide").unwrap();
    repo.filter_by_tag(&main_filter.into());

    edit_or_create(
        FromStr::from_str("test").unwrap(),
        &mut repo,
        None,
        EditTagArgs {
            content: FromStr::from_str("gg").unwrap(),
            change_existing: true,
            append_namespace: true,
        },
    )
    .await
    .expect_err("沒有 BANG! 就找到編輯的腳本！？");

    let (p, e) = edit_or_create(
        FromStr::from_str("test!").unwrap(),
        &mut repo,
        None,
        EditTagArgs {
            content: FromStr::from_str("+a,^b,c").unwrap(),
            change_existing: true,
            append_namespace: true,
        },
    )
    .await
    .unwrap();

    assert_eq!(p, get_home().join("test.sh"));

    let mut tags = std::collections::HashSet::<Tag>::new();
    tags.insert(FromStr::from_str("a").unwrap());
    tags.insert(FromStr::from_str("c").unwrap());
    tags.insert(FromStr::from_str("hide").unwrap());
    assert_eq!(tags, e.tags);
}
