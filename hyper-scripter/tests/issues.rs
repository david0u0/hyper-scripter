#[allow(dead_code)]
#[path = "tool.rs"]
mod tool;

use std::fs::remove_file;
use tool::*;

#[test]
fn test_mv_same_name() {
    println!("移動腳本時，若前後腳本的路徑相同（腳本分類所致），應順利改動");
    let _g = setup();

    run!("e test1 | echo 1").unwrap();
    run!("e test2 | echo 2").unwrap();

    run!("mv test1 -T tmux").unwrap();
    assert_eq!(run!("test1").unwrap(), "1");

    run!("mv test1 test2").expect_err("移動成撞名的腳本應報錯");
    assert_eq!(run!("test1").unwrap(), "1");
    run!("mv test1 test2 -T rb").expect_err("移動成撞名的腳本，即使路徑不同，也應報錯");
    assert_eq!(run!("test1").unwrap(), "1");
}
#[test]
fn test_cp_same_name() {
    println!("複製腳本時，若和既存的腳本撞名，應報錯");
    let _g = setup();

    run!("e test1 -T rb | puts 1").unwrap();
    run!("e test2 | echo 2").unwrap();
    run!("e test3 -T rb | puts 3").unwrap();

    run!("cp test1 test3").expect_err("複製成撞名的腳本應報錯");
    assert_eq!(run!("test3").unwrap(), "3");

    run!("cp test1 test2").expect_err("複製成撞名的腳本，即使路徑不同，也應報錯");
    assert_eq!(run!("test2").unwrap(), "2");

    assert_eq!(run!("test1").unwrap(), "1");
}
#[test]
fn test_rm_non_exist() {
    println!("若欲刪除的腳本不存在，應直接消滅之。");
    let _g = setup();

    run!("e test1 | echo '刪我啊'").unwrap();
    run!("e test2 | echo '別刪我QQ'").unwrap();
    assert_eq!(run!("test1").unwrap(), "刪我啊");
    let file = get_home().join("test1.sh");
    remove_file(&file).unwrap();
    assert_ls_len(2, None, None);
    assert_eq!(run!("which test1").unwrap(), file.to_string_lossy()); // TODO: 應該允許 which 嗎？

    run!("test1").expect_err("刪掉的腳本還能執行！？");
    run!("rm *").expect("rm 應該要消滅掉不存在的腳本");

    assert_ls_len(1, Some("all"), None);
}
#[test]
fn test_hs_in_hs() {
    println!("若腳本甲裡呼叫了本程式去執行腳本乙，完成之後腳本甲的時間應較新");
    let _g = setup();

    let outer = ScriptTest::new("outer", None, Some("$HS_EXE -H $HS_HOME inner"));
    let inner = ScriptTest::new("inner", None, Some("echo 我在第幾層？"));

    inner.can_find("^2").unwrap_err();
    outer.run("").unwrap();
    inner.can_find("^2").unwrap();

    outer.filter("--humble").run("").unwrap();
    inner.can_find("^2").unwrap_err();

    let outer_humble = ScriptTest::new(
        "outer-humble",
        None,
        Some("$HS_EXE -H $HS_HOME history humble $HS_RUN_ID && $HS_EXE -H $HS_HOME inner"),
    );

    inner.can_find("-").unwrap_err();
    outer_humble.run("").unwrap();
    inner.can_find("-").unwrap();
}

#[test]
fn test_remove_history_in_script() {
    println!("在腳本執行途中砍掉執行歷史，則該腳本的「執行完畢」事件應該一併消失");
    let _g = setup();

    run!("e test1 | echo 1").unwrap();
    run!("e test2 | echo 2 && $HS_EXE -H $HS_HOME history rm =${{NAME}}! 1").unwrap();

    assert_eq!(run!("-").unwrap(), "2");
    assert_eq!(run!("-").unwrap(), "2"); // 比較晚創造，所以刪了執行事件還是腳本2先
    assert_eq!(run!("test1").unwrap(), "1");
    assert_eq!(run!("test2").unwrap(), "2");
    assert_eq!(run!("-").unwrap(), "1");
}

#[test]
fn test_edit_existing_bang() {
    println!("用 BANG! 編輯已存在的腳本，不該出錯");
    let _g = setup();

    run!("e test -t hide | echo 躲貓貓").unwrap();

    // 當場變一個異步執行期出來。不要直接把測試函式寫成異步，否則 setup 中鎖的處理會出問題…
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        use hyper_scripter::error::{Error, RedundantOpt};
        use hyper_scripter::script_repo::ScriptRepo;
        use hyper_scripter::tag::{Tag, TagFilter};
        use hyper_scripter::util::{
            init_env,
            main_util::{edit_or_create, EditTagArgs},
        };

        let mut repo = {
            let (env, _) = init_env(true).await.unwrap();
            ScriptRepo::new(None, env).await.unwrap()
        };
        repo.filter_by_tag(&"all,^hide".parse::<TagFilter>().unwrap().into());

        fn assert_tags<'a>(expect: &[&str], actual: impl Iterator<Item = &'a Tag>) {
            let expect: Vec<Tag> = expect.iter().map(|s| s.parse().unwrap()).collect();
            let actual: Vec<Tag> = actual.map(|s| s.clone()).collect();
            assert_eq!(expect, actual);
        }
        macro_rules! try_edit {
            ($query:expr, $ty:expr, $tag:expr) => {
                edit_or_create(
                    $query.parse().unwrap(),
                    &mut repo,
                    $ty.map(|s: &str| s.parse().unwrap()),
                    EditTagArgs {
                        content: $tag.parse().unwrap(),
                        explicit_tag: false,
                        explicit_filter: false,
                    },
                )
            };
        }

        let err = try_edit!("test", Some("rb"), "gg")
            .await
            .expect_err("沒有 BANG! 就找到編輯的腳本！？");
        assert!(matches!(err, Error::ScriptIsFiltered(s) if s == "test"));

        let err = try_edit!("=test", Some("rb"), "gg").await.unwrap_err();
        assert!(matches!(err, Error::ScriptIsFiltered(s) if s == "test"));

        let err = try_edit!("test!", Some("rb"), "gg").await.unwrap_err();
        assert!(matches!(err, Error::RedundantOpt(RedundantOpt::Type)));

        let (p, e, sub) = try_edit!("tes", Some("rb/traverse"), "+gg").await.unwrap();
        assert_eq!(p, get_home().join("tes.rb"));
        assert!(sub.is_some());
        assert_tags(&["gg"], e.tags.iter());

        let (p, e, sub) = try_edit!("test2", None, "+gg").await.unwrap();
        assert_eq!(p, get_home().join("test2.sh"));
        assert!(sub.is_none());
        assert_tags(&["gg"], e.tags.iter());

        let (p, e, sub) = try_edit!("test!", None, "+a,^b,c").await.unwrap();
        assert_eq!(p, get_home().join("test.sh"));
        assert!(sub.is_none());
        assert_tags(&["hide"], e.tags.iter());
    });
}

// TODO: edit wild & edit phantom

#[test]
fn test_edit_without_change() {
    println!("沒有動到檔案就不要記錄寫入事件");
    let _g = setup();

    let t = ScriptTest::new("target", None, None);
    let base = ScriptTest::new("baseline", None, None);
    base.can_find("!").unwrap();
    run!("history neglect {}", t.get_name()).unwrap();
    t.can_find_by_name().expect_err("被忽略還找得到？");
    run!("e {}!", t.get_name()).unwrap(); // nothing changed!
    t.can_find_by_name().expect_err("空編輯不應打破時間篩選");

    base.can_find("-").unwrap();
    t.can_find("!").unwrap();

    run!("e {}! | echo $NAME", t.get_name()).unwrap(); // changed!
    t.can_find_by_name().unwrap();
    assert_eq!(
        format!("{}\n{}", t.get_name(), t.get_name()),
        run!("-").unwrap(),
        "帶內容編輯應打破時間篩選"
    );

    let orphan = ScriptTest::new("orphan", None, Some(""));
    orphan
        .filter("-a")
        .can_find_by_name()
        .expect_err("空編輯新腳本應該要被砍掉");
}

#[test]
fn test_multifuzz() {
    use hyper_scripter::{fuzzy::*, SEP};

    println!("模糊搜撞在一起時的特殊行為 1. 取最新者 2. 不可為「正解」的後綴");
    let _g = setup();
    let pref = ScriptTest::new("multifuzz", None, None);
    let t1 = ScriptTest::new("multifuzz/t1", None, None);
    let t2 = ScriptTest::new("multifuzz/t2", None, None);

    // 當場變一個異步執行期出來。不要直接把測試函式寫成異步，否則 setup 中鎖的處理會出問題…
    let rt = tokio::runtime::Runtime::new().unwrap();
    let res = rt.block_on(async {
        fuzz("mult", [&t1, &t2, &pref].iter().map(|t| t.get_name()), SEP)
            .await
            .unwrap()
            .unwrap()
    });
    {
        let is_match =
            matches!(&res, Multi{ans, others, ..} if *ans == pref.get_name() && others.len() == 2);
        assert!(is_match, "{:?} 並非預期中結果，應更新測資", res);
    }

    t2.can_find("multifuzz/t").unwrap();
    t1.run("").unwrap();
    t1.can_find("multifuzz/t").unwrap();
    t2.run("").unwrap();
    t2.can_find("multifuzz/t").unwrap();

    pref.can_find("multifuzz").unwrap();
}

#[test]
fn test_history_rm_range() {
    println!("移除一整段事件");
    let _g = setup();

    let t2 = ScriptTest::new("test2", None, None);
    t2.run("sep").unwrap();
    t2.run("a").unwrap();
    t2.run("b").unwrap();

    let t1 = ScriptTest::new("test", None, None);
    t1.run("sep").unwrap();
    t1.run("a").unwrap();
    t1.run("b").unwrap();
    t1.run("sep").unwrap();
    t2.run("b").unwrap(); // t2 而非 t1
    t1.run("a").unwrap();
    t1.run("b").unwrap();

    fn show_history(script: &ScriptTest) -> Vec<String> {
        let s = run!("history show {}", script.get_name()).unwrap();
        s.split('\n').map(|s| s.to_owned()).collect()
    }

    t1.can_find("-").expect("用最近期詢問找不到？");
    assert_eq!(show_history(&t1), vec!["b", "a", "sep"]);

    run!("history rm {} 1..3", t1.get_name()).unwrap(); // rm b & a
    assert_eq!(show_history(&t1), vec!["sep"]);

    t2.can_find("-").expect("刪除整段事件未能影響近期詢問");

    assert_eq!(
        show_history(&t2),
        vec!["b", "a", "sep"],
        "另一個腳本的歷史爛掉了"
    );
}

#[test]
fn test_fuzz_dot_or_endwith_slash() {
    println!("測試以`/`結尾的腳本名，以及`.`腳本名");
    let _g = setup();

    let t = ScriptTest::new("test/slash", None, None);
    t.can_find("test/").unwrap();
    t.can_find("t/").unwrap();
    t.can_find("slash/").unwrap();

    t.can_find("sla//").expect_err("兩個`/`結尾仍不可行");
    t.can_find("/sla").unwrap_err();

    run!("e illegal/").expect_err("不應創建以`/`結尾的腳本");

    run!("e .").expect_err("不應創建名為`.`的腳本");
    let t = ScriptTest::new(".1", None, None);
    run!("e .").expect("應查詢到剛才建出的腳本");
    t.can_find("1").unwrap();
    t.can_find(".1").unwrap();
    t.can_find(".").unwrap();

    t.can_find("..").expect_err("兩個`.`仍不可行");
    t.can_find(".a").unwrap_err();

    run!("mv - .").expect_err("不應創建名為`.`的腳本");
}

#[test]
fn test_existing_path() {
    println!("測試路徑衝突的邊角案例");
    let _g = setup();

    let _ = ScriptTest::new("dir/file", None, None);
    run!("e =dir -T txt | echo 1").expect_err("與目錄撞路徑");
    run!("e =dir/file.sh -T txt | echo 1").expect_err("與既存腳本撞路徑");
    run!("e =dir/file.sh/file | echo 1").unwrap_err();

    assert_ls_len(1, Some("all"), None);
}
