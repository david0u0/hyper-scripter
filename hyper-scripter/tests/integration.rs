#[allow(dead_code)]
#[path = "tool.rs"]
mod tool;

use hyper_scripter::{
    path::{normalize_path, HS_REDIRECT},
    util::main_util::prepare_pre_run,
};
use std::fs::write;
use tool::*;

const MSG: &str = "你好，腳本人！";
const MSG_JS: &str = "你好，爪哇腳本人！";

#[test]
fn test_tags() {
    let _g = setup();
    run!("e ? | echo \"{}\"", MSG).unwrap();
    assert_eq!(MSG, run!("-").unwrap());

    run!(
        "e -t +super_tag,hide test/js -T js | console.log(\"{}\")",
        MSG_JS
    )
    .unwrap();
    run!("tesjs").expect_err("標籤沒有篩選掉不該出現的腳本！");
    assert_eq!(MSG_JS, run!("-s super_tag -").unwrap());

    assert_eq!(MSG, run!(".1").expect("標籤篩選把舊的腳本搞爛了！"));

    run!("tesjs").expect_err("標籤沒有篩選掉不該出現的腳本！可能是上上個操作把設定檔寫爛了");
    run!("tags --name no-hidden all").unwrap();
    run!("tesjs").expect("沒吃到設定檔的標籤？");

    run!("tags --name no-hidden all,^hide!").unwrap();
    run!("tesjs").expect_err("沒吃到設定檔的標籤？");

    run!("tags all").unwrap();
    run!("tesjs").expect("沒吃到設定檔的標籤？");

    run!("tags test").unwrap();
    run!("tesjs").expect_err("命名空間變成標籤了？");
}

#[test]
fn test_help() {
    let _g = setup();
    let help_msg = "This is a help message";
    run!("e --no-template ? | # [HS_HELP]: {}", help_msg).unwrap();
    assert_eq!(run!("help -").unwrap(), help_msg);
    assert_ne!(run!("help").unwrap(), help_msg);
    assert_ne!(run!("help mv").unwrap(), help_msg);
    run!("help a-non-existing-script").expect_err("不該找到腳本");
}

#[test]
fn test_mv_cp() {
    let _g = setup();

    run!("e -t test ? -T js --no-template | echo $HS_TAGS").unwrap();
    run!("-").expect_err("用 nodejs 執行 echo ……？");

    run!("mv 1 -T sh -t test2").unwrap();
    assert_eq!("test2", run!("-").unwrap());
    assert!(
        check_exist(&[".resource/anonymous", "1.sh"]),
        "改腳本類型失敗"
    );
    assert!(
        !check_exist(&[".resource/anonymous", "1.js"]),
        "改了腳本類型舊檔案還留著？"
    );

    run!("mv 1").expect("應該允許空的 mv，作為 touch 的功能");
    run!("mv 1 -t +hide").unwrap();
    run!("-").expect_err("用 mv 修改標籤失敗？");

    run!("cp -s hide 1 -t +cp .2").unwrap();
    let mut res: Vec<_> = run!(".2!")
        .unwrap()
        .split(' ')
        .map(|s| s.to_string())
        .collect();
    res.sort();
    assert_eq!(vec!["cp", "hide", "test2"], res);

    run!("cp -s hide - -t only .3").unwrap();
    let res: Vec<_> = run!("!")
        .unwrap()
        .split(' ')
        .map(|s| s.to_string())
        .collect();
    assert_eq!(vec!["only"], res);

    // TODO: mv and cp existing
}

const TALKER: &str = "--腳本小子";
const APPEND: &str = "第二行";
#[test]
fn test_run() {
    let _g = setup();
    run!("e test-with-args | echo -e \"$1：{}\n$2\"", MSG).unwrap();
    assert_eq!(
        format!("{}：{}\n{}", TALKER, MSG, APPEND),
        run!("- {} {}", TALKER, APPEND).unwrap(),
        "沒吃到命令行參數？"
    );
}

#[test]
fn test_exact() {
    let _g = setup();
    run!("e test-exact | echo 'test exact!'").unwrap();
    run!("tesct").expect("模糊搜不到東西！");
    run!("=tesct").expect_err("打錯名字卻還搜得到！");
    run!("=test-exact").expect("打完整名字卻搜不到！");
}

#[test]
fn test_prev() {
    let _g = setup();

    run!("e test-prev1 | echo 'test prev 1'").unwrap();
    run!("e test-prev2 | echo 'test prev 2'").unwrap();
    run!("e test-prev3 --no-template | echo 'test prev 3'").unwrap();

    assert_eq!(run!("^2").unwrap(), "test prev 2");
    assert_eq!(run!("^2").unwrap(), "test prev 3");
    assert_eq!(run!("^^^").unwrap(), "test prev 1");
    assert_eq!(run!("cat ^2").unwrap(), "echo 'test prev 3'");
    assert_eq!(
        run!("-").unwrap(),
        "test prev 3",
        "cat 沒有確實影響到腳本時序"
    );

    run!("^^^^").expect_err("明明只有三個腳本，跟我說有第四新的？");
}

#[test]
fn test_edit_same_name() {
    let _g = setup();
    run!("e i-am-hidden -t hide | echo \"{}\"", MSG).unwrap();
    run!("-").expect_err("執行了隱藏的腳本？？");
    run!("e i-am-hidden yo | echo I'm screwed QQ").expect_err("竟然能編輯撞名的腳本？");
    assert_eq!(MSG, run!("-s hide -").unwrap(), "腳本被撞名的編輯搞爛了？");
}

#[test]
fn test_edit_append() {
    let _g = setup();

    run!("e -t test test | echo 第一行").unwrap();
    run!("e - | echo 第二行\necho 第三行").unwrap();
    assert_eq!("第一行\n第二行\n第三行", run!("-s test -").unwrap());
}

#[test]
fn test_edit_with_tag() {
    let _g = setup();

    fn msg(i: i32) -> String {
        format!("你好，{}號腳本人！", i)
    }

    run!("tags innate").unwrap();

    run!("e test1 -t tag2 | echo \"{}\"", msg(1)).unwrap();
    run!("-s innate -").expect_err("吃到了不該吃的標籤！");
    assert_eq!(msg(1), run!("-s tag2 -").unwrap());

    run!("e test2 -t +tag2,tag3 | echo \"{}\"", msg(2)).unwrap();
    assert_eq!(msg(2), run!("-s innate -").unwrap());
    assert_eq!(msg(2), run!("-s tag2 -").unwrap());
    assert_eq!(msg(2), run!("-s tag3 -").unwrap());
}

#[test]
fn test_multi_select() {
    let _g = setup();
    run!("e nobody | echo \"{}\"", MSG).unwrap();
    run!("e -t test,pin test-pin | echo \"{}\"", MSG).unwrap();
    run!("e -t pin pin-only | echo \"{}\"", MSG).unwrap();

    assert_eq!(MSG, run!("pin-only").unwrap());
    assert_eq!(MSG, run!("test-pin").unwrap());
    assert_eq!(MSG, run!("nobody").unwrap());

    run!("tags +hidden").unwrap();
    assert_eq!(MSG, run!("pin-only").unwrap());
    assert_eq!(MSG, run!("test-pin").unwrap());
    run!("nobody").expect_err("未能被主篩選器篩掉");

    run!("tags +^test").unwrap();
    assert_eq!(MSG, run!("pin-only").unwrap());
    run!("test-pin").expect_err("未能被主篩選器篩掉");

    assert_eq!(MSG, run!("-a test-pin").unwrap());
}

#[test]
fn test_rm() {
    let _g = setup();
    run!("e longlive | echo 矻立不搖").unwrap();

    run!("e test/ya -t test-tag | echo \"{}\"", MSG).unwrap();
    assert_eq!(MSG, run!("test/ya").unwrap());
    run!("e ? | echo \"你匿\"").unwrap();
    assert_eq!("你匿", run!(".1").unwrap());

    run!("rm - test*").unwrap();
    run!("test/ya").expect_err("未能被刪除掉");
    run!(".1").expect_err("未能被刪除掉");
    run!("-a test/ya").expect_err("被刪除掉的腳本竟能用 `-a` 找回來");
    assert_eq!(MSG, run!("-s remove test").unwrap());
    assert_eq!(
        MSG,
        run!("-s test-tag test").expect("刪除沒有保留本來的標籤？")
    );

    run!("-s remove,^test-tag -").expect_err("匿名腳本該真的被刪掉！");

    assert_eq!("矻立不搖", run!("longlive").unwrap());

    run!("e my/namespace/super-test | echo \"不要刪我 QmmmmQ\"").unwrap();
    assert_eq!("不要刪我 QmmmmQ", run!("mysuper-test").unwrap());
    run!("rm mysupertest").expect("刪除被命名空間搞爛了");
    run!("mysuper-test").expect_err("未能被刪除掉");
    assert_eq!(
        "不要刪我 QmmmmQ",
        run!("-s remove my/namespace/super-test").unwrap()
    );

    assert_eq!("矻立不搖", run!("longlive").unwrap());

    assert!(check_exist(&["longlive.sh"]));
    run!("rm * -s all --purge").expect("未能消滅掉一切");

    assert!(!check_exist(&["longlive.sh"]));
    run!("-s all which").expect_err("沒有確實消滅掉一切");

    // NOTE: ---- 測試用 ! 來刪除 ----
    run!("e -t hide hidden | echo 隱藏腳本，請用 ! 刪除我").unwrap();
    run!("rm --purge hidden").expect_err("沒加 ! 就刪掉了？");
    run!("rm --purge hidden!").expect("用了 ! 沒刪成功？");
    run!("rm --purge hidden!").expect_err("連續刪兩次？");
}

#[test]
fn test_namespace_reorder_search() {
    let _g = setup();
    run!("e my/super/long/namespace-d/test-script | echo \"{}\"", MSG).unwrap();
    run!("e a/shorter/script -T js | console.log(\"{}\")", MSG_JS).unwrap();
    assert_eq!(MSG, run!("myscript").expect("正常空間搜尋失敗"));
    assert_eq!(MSG, run!("scriptsuper").expect("重排命名空間搜尋失敗"));
    assert_eq!(MSG, run!("testlong").expect("重排命名空間搜尋失敗"));
    assert_eq!(MSG_JS, run!("scrishorter").expect("重排命名空間搜尋失敗"));
    assert_eq!(MSG, run!("namsplongsuery").expect("重排命名空間搜尋失敗"));
    run!("script-test").expect_err("重排到腳本名字去了= =");
}

#[test]
fn test_append_tags() {
    let _g = setup();
    const CONTENT: Option<&str> = Some("echo $HS_TAGS");
    run!("tags global,test").unwrap();

    pub fn assert_tags<const N: usize>(script: &ScriptTest, tags: [&str; N]) {
        let res = script.select("-a").run("").unwrap();
        let mut actual_tags: Vec<_> = res.split(' ').filter(|s| !s.is_empty()).collect();
        actual_tags.sort();
        let mut expected_tags: Vec<_> = tags.iter().map(|s| *s).collect();
        expected_tags.sort();
        assert_eq!(expected_tags, actual_tags, "{:?} 的標籤不如預期", script);
    }

    let append_test = ScriptTest::new("append-test", Some("+append"), CONTENT);
    let no_append_test = ScriptTest::new("no-append-test", Some("no-append"), CONTENT);

    assert_tags(&append_test, ["global", "test", "append"]);
    no_append_test.assert_not_exist(None, None);
    assert_tags(&no_append_test, ["no-append"]);

    run!("mv -s no-append no-append-test -t +eventually-append").unwrap();

    assert_tags(&no_append_test, ["no-append", "eventually-append"]);
    assert_tags(&append_test, ["global", "test", "append"]);
    // 測試 ^all 的功能（無視一切先前的標籤）
    let t1 = ScriptTest::new("no-tag", Some("+^all"), CONTENT);
    let t2 = ScriptTest::new("so-normal", None, CONTENT);
    assert_tags(&t1, []);
    assert_tags(&t2, ["global", "test"]);
    // 測試 ^{some-tag} 的功能
    let t1 = ScriptTest::new("only-test", Some("+^global"), CONTENT);
    let t2 = ScriptTest::new("only-global", Some("+^test"), CONTENT);
    let t3 = ScriptTest::new("nooooo", Some("^test"), CONTENT);
    assert_tags(&t1, ["test"]);
    assert_tags(&t2, ["global"]);
    assert_tags(&t3, []);
}

#[test]
fn test_bang() {
    let _g = setup();
    let first_file = get_home().join("hidden_first.sh");
    let fourth_file = get_home().join("fourth.sh");

    run!("e -t hide hidden_first | echo $0").unwrap();
    run!("e -t hide hidden_second | echo 第二").unwrap();
    run!("e -t hide hidden_third | echo 第三").unwrap();
    run!("cp firs! fourth").unwrap();
    run!("mv -s tag4 four! -t all").unwrap();

    run!("first").expect_err("執行了隱藏的腳本？？");
    assert_eq!(first_file.to_string_lossy(), run!("firt!").unwrap());
    assert_eq!("第二", run!("seco!").unwrap());
    assert_eq!("第二", run!("!").unwrap());
    assert_eq!(fourth_file.to_string_lossy(), run!("-").unwrap());
    assert_eq!("第三", run!("=hidden_third!").unwrap());
    assert_eq!(fourth_file.to_string_lossy(), run!("four!").unwrap());

    assert_ls(
        vec!["hidden_first", "fourth"],
        Some("+^tag4"),
        Some("firs! fourth"),
    );
}

#[test]
fn test_redirect() {
    let _g = setup();
    // NOTE: 重導向若為相對路徑，則其基準是當前的腳本之家
    let redirected = "../.hyper_scripter_redirect";
    let redirected_abs = normalize_path(get_home().join(redirected)).unwrap();

    match std::fs::remove_dir_all(&redirected_abs) {
        Ok(_) => (),
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                panic!("重整重導向用資料夾失敗了……")
            }
        }
    }

    write(get_home().join(HS_REDIRECT), redirected).unwrap();
    run!("e --fast test | echo $(realpath $(dirname $0))").unwrap();
    assert_eq!(run!("-").unwrap(), redirected_abs.to_string_lossy());
}

#[test]
fn test_mandatory_select() {
    let _g = setup();
    let t1 = ScriptTest::new("prj1/t", Some("prj1"), None);
    let t2 = ScriptTest::new("prj2/t", Some("prj2"), None);
    let t3 = ScriptTest::new("prj1/src/t", Some("prj1,src"), None);
    let t4 = ScriptTest::new("prj2/src/t", Some("prj2,src"), None);
    let t5 = ScriptTest::new("hide/prj2/src/t", Some("hide,prj2,src"), None);

    assert_ls(vec![&t1, &t2, &t3, &t4], None, None);
    assert_ls(Vec::<String>::new(), None, Some("-s all!"));
    // NOTE: 順便測試在參數上帶多個篩選器
    assert_ls(vec![&t3, &t4], None, Some("-s +src!"));
    assert_ls(vec![&t3, &t4], None, Some("-s +src! -s +prj1")); // +prj1 非強制，不影響
    assert_ls(vec![&t3], None, Some("-s +src! -s +prj1!")); // +prj1! 為強制，會影響

    run!("tags +prj1").unwrap();
    assert_ls(vec![&t1, &t3], None, None);
    assert_ls(vec![&t3, &t4, &t5], Some("src"), None);
    assert_ls(vec![&t1, &t3, &t4], Some("+src"), None);
    assert_ls(vec![&t3], Some("+src!"), None);
    assert_ls(vec![&t4, &t5], None, Some("-s all -s +src! -s +prj2!"));
}

#[test]
fn test_custom_env() {
    let _g = setup();

    let mut conf = load_conf();
    conf.env
        .insert("HOME_N_NAME".to_owned(), "{{home}}::{{name}}".to_owned());
    conf.store().unwrap();

    run!("e myname | echo $HOME_N_NAME").unwrap();
    assert_eq!(
        run!("-").unwrap(),
        format!("{}::myname", get_home().to_string_lossy())
    );
}

#[test]
fn test_prerun() {
    let _g = setup();

    prepare_pre_run(Some(
        "#!/usr/bin/env ruby\nputs \"測試預腳本=_= #{ENV['NAME']} #{ARGV.join(',')}\"",
    ))
    .unwrap();
    run!("e myname | echo 實際執行=_=").unwrap();
    assert_eq!(
        run!("- 參數1 參數2").unwrap(),
        "測試預腳本=_= myname 參數1,參數2\n實際執行=_="
    );
}

#[test]
fn test_ls_query() {
    let _g = setup();

    fn create(name: &str, tag: Option<&str>) {
        let tag = tag.map(|t| format!("-t {}", t)).unwrap_or_default();
        run!("e ={}! {} | echo dummy", name, tag).expect(&format!("創建 {} 失敗", name));
    }

    create("fuzzed/not-shown", None);
    create("fuzzed/shown", None);

    create("not-shown", None);

    create("wildcard1", None);
    create("wildcard2", None);

    create("prev", None);

    create("hide/not-shown", Some("hide"));
    create("hide/exact", Some("hide"));
    create("hide/fuzz", Some("hide"));
    create("hide/prev", Some("hide"));

    assert_ls(
        vec![
            "fuzzed/shown",
            "wildcard1",
            "wildcard2",
            "prev",
            "hide/exact",
            "hide/fuzz",
            "hide/prev",
        ],
        None,
        Some("showfuz wildcar* - =hide/exact! fzhid! !"),
    );
}

#[test]
fn test_miss_event() {
    let _g = setup();
    let hidden = ScriptTest::new("1", Some("hide"), None);
    let neglected = ScriptTest::new("3", None, None);
    let normal = ScriptTest::new("2", None, None);
    run!("history neglect {}", neglected.get_name()).unwrap();

    let test_all = || {
        hidden.can_find("!").unwrap_err();
        hidden.can_find_by_name().unwrap_err();
        hidden.can_find("!").expect("錯過事件無效？");

        neglected.can_find("!").unwrap_err();
        neglected.can_find_by_name().expect_err("neglect 無效？");
        neglected.can_find("!").expect("錯過事件無效？");
        neglected
            .can_find_by_name()
            .expect_err("錯過事件打破了時間篩選器？");

        normal.can_find_by_name().unwrap();
        normal.can_find("-").unwrap();
        normal.can_find("!").expect_err("亂製造錯過事件？");
    };

    test_all();
    test_all();
}
