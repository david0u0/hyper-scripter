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

    run("mv test -T tmux").unwrap();
    assert_eq!(run("test").unwrap(), "1");

    run("mv test test2").expect_err("移動成撞名的腳本應報錯");
    assert_eq!(run("test").unwrap(), "1");
    run("mv test test2 -T rb").expect_err("移動成撞名的腳本，即使分類不同，也應報錯");
    assert_eq!(run("test").unwrap(), "1");
}
#[test]
fn test_cp_same_name() {
    println!("複製腳本時，若和既存的腳本撞名，應報錯");
    let _g = setup();

    run("e test -T rb | puts 1").unwrap();
    run("e test2 | echo 2").unwrap();
    run("e test3 -T rb | puts 3").unwrap();

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
    assert_ls_len(2, None, None);
    assert_eq!(run("which test").unwrap(), file.to_string_lossy()); // TODO: 應該允許 which 嗎？

    run("test").expect_err("刪掉的腳本還能執行！？");
    run("rm *").expect("rm 應該要消滅掉不存在的腳本");

    assert_ls_len(1, Some("all"), None);
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

#[test]
fn test_remove_history_in_script() {
    println!("在腳本執行途中砍掉執行歷史，則該腳本的「執行完畢」事件應該一併消失");
    let _g = setup();

    run("e test1 | echo 1").unwrap();
    run("e test2 | echo 2 && $HS_EXE -H $HS_HOME history rm =${NAME}! 1").unwrap();

    assert_eq!(run("-").unwrap(), "2");
    assert_eq!(run("-").unwrap(), "2"); // 比較晚創造，所以刪了執行事件還是腳本2先
    assert_eq!(run("test1").unwrap(), "1");
    assert_eq!(run("test2").unwrap(), "2");
    assert_eq!(run("-").unwrap(), "1");
}

#[test]
fn test_edit_existing_bang() {
    println!("用 BANG! 編輯已存在的腳本，不該出錯");
    let _g = setup();

    run("e test -t hide | echo 躲貓貓").unwrap();

    // 當場變一個異步執行期出來。不要直接把測試函式寫成異步，否則 setup 中鎖的處理會出問題…
    let mut rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        use hyper_scripter::error::Error;
        use hyper_scripter::script_repo::ScriptRepo;
        use hyper_scripter::tag::{Tag, TagFilter};
        use hyper_scripter::util::main_util::{edit_or_create, EditTagArgs};
        use hyper_scripter_historian::Historian;

        let mut repo = {
            let historian = Historian::new(get_home().to_owned()).await.unwrap();
            let (pool, _) = hyper_scripter::db::get_pool().await.unwrap();
            ScriptRepo::new(pool, None, historian, false).await.unwrap()
        };
        repo.filter_by_tag(&"all,^hide".parse::<TagFilter>().unwrap().into());

        let err = edit_or_create(
            "test".parse().unwrap(),
            &mut repo,
            Some("rb".parse().unwrap()), // 即使不同型的腳本也該報錯
            EditTagArgs {
                content: "gg".parse().unwrap(),
                append_namespace: true,
                explicit_tag: false,
                explicit_filter: false,
            },
        )
        .await
        .expect_err("沒有 BANG! 就找到編輯的腳本！？");
        matches!(err, Error::ScriptIsFiltered(s) if s == "test");

        let (p, e) = edit_or_create(
            "test!".parse().unwrap(),
            &mut repo,
            None,
            EditTagArgs {
                content: "+a,^b,c".parse().unwrap(),
                append_namespace: true,
                explicit_tag: false,
                explicit_filter: false,
            },
        )
        .await
        .unwrap();

        assert_eq!(p, get_home().join("test.sh"));
        use fxhash::FxHashSet as HashSet;
        let mut tags = HashSet::<Tag>::default();
        tags.insert("hide".parse().unwrap());
        assert_eq!(tags, e.tags);
    });
}

// TODO: edit wild & edit phantom

#[test]
fn test_edit_without_change() {
    println!("沒有動到檔案就不要記錄寫入事件");
    let _g = setup();

    run("e test1 | echo $NAME").unwrap();
    run("e test2 | echo $NAME").unwrap();
    assert_eq!("test2", run("-").unwrap());
    run("e test1").unwrap(); // nothing changed!
    assert_eq!("test2", run("-").unwrap());
    run("e test1 | echo $NAME").unwrap();
    assert_eq!("test1\ntest1", run("-").unwrap());
}
