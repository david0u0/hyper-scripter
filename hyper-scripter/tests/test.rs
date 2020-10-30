#![feature(str_split_once)]

#[allow(dead_code)]
#[path = "../../hyper-scripter-test-lib/test_util.rs"]
mod test_util;

use hyper_scripter::path::HS_EXECUTABLE_INFO_PATH;
use regex::Regex;
use test_util::*;
const MSG: &'static str = "你好，腳本人！";

const MSG_JS: &'static str = "你好，爪哇腳本人！";
#[test]
fn test_tags() {
    let _g = setup();
    run(&format!("e . | echo \"{}\"", MSG)).unwrap();
    assert_eq!(MSG, run("-").unwrap());

    run(&format!(
        "-f super_tag,hide e test/js -c js | console.log(\"{}\")",
        MSG_JS
    ))
    .unwrap();
    run("tesjs").expect_err("標籤沒有篩選掉不該出現的腳本！");
    assert_eq!(MSG_JS, run("-f super_tag -").unwrap());

    assert_eq!(MSG, run(".1").expect("標籤篩選把舊的腳本搞爛了！"));

    run("tesjs").expect_err("標籤沒有篩選掉不該出現的腳本！可能是上上個操作把設定檔寫爛了");
    run("tags all").unwrap();
    run("tags no-hidden=all").unwrap();
    run("tesjs").expect("沒吃到設定檔的標籤？");
    run("tags test").unwrap();
    run("tesjs").expect("命名空間沒賦與它標籤？");
}

#[test]
fn test_mv() {
    let _g = setup();

    run(&format!("e . -c js --no-template | echo \"{}\"", MSG)).unwrap();
    run("-").expect_err("用 nodejs 執行 echo ……？");

    run("mv 1 -c sh").unwrap();
    assert_eq!(MSG, run("-").unwrap());
    assert!(check_exist(&[".anonymous", "1.sh"]), "改腳本類型失敗");
    assert!(
        !check_exist(&[".anonymous", "1.js"]),
        "改了腳本類型舊檔案還留著？"
    );

    run("mv 1 -t hide").unwrap();
    run("-").expect_err("用 mv 修改標籤失敗？");
}

const TALKER: &'static str = "--腳本小子";
const APPEND: &'static str = "第二行";
#[test]
fn test_run() {
    let _g = setup();
    run(&format!("e test-with-args | echo -e \"$1：{}\n$2\"", MSG)).unwrap();
    assert_eq!(
        format!("{}：{}\n{}", TALKER, MSG, APPEND),
        run(&format!("- {} {}", TALKER, APPEND)).unwrap(),
        "沒吃到命令行參數？"
    );

    assert_eq!(
        read(&[HS_EXECUTABLE_INFO_PATH]),
        get_exe_abs(),
        "記錄到的執行檔位置有誤"
    );
}

#[test]
fn test_exact() {
    let _g = setup();
    run("e test-exact | echo 'test exact!'").unwrap();
    run("tesct").expect("模糊搜不到東西！");
    run("=tesct").expect_err("打錯名字卻還搜得到！");
    run("=test-exact").expect("打完整名字卻搜不到！");
}

#[test]
fn test_prev() {
    let _g = setup();

    run("e test-prev1 | echo 'test prev 1'").unwrap();
    run("e test-prev2 | echo 'test prev 2'").unwrap();
    run("e test-prev3 -n | echo 'test prev 3'").unwrap();

    assert_eq!(run("^2").unwrap(), "test prev 2");
    assert_eq!(run("^2").unwrap(), "test prev 3");
    assert_eq!(run("^^^").unwrap(), "test prev 1");
    assert_eq!(run("cat ^2").unwrap(), "echo 'test prev 3'");
    assert_eq!(
        run("-").unwrap(),
        "test prev 3",
        "cat 沒有確實影響到腳本時序"
    );

    run("^^^^").expect_err("明明只有三個腳本，跟我說有第四新的？");
}

#[test]
fn test_edit_same_name() {
    let _g = setup();
    run(&format!("e i-am-hidden -t hide | echo \"{}\"", MSG)).unwrap();
    run("-").expect_err("執行了隱藏的腳本？？");
    run("e i-am-hidden yo").expect_err("竟然能編輯撞名的腳本？");
    assert_eq!(MSG, run("-f hide -").unwrap(), "腳本被撞名的編輯搞爛了？");
}

#[test]
fn test_edit_with_tag() {
    let _g = setup();

    fn msg(i: i32) -> String {
        format!("你好，{}號腳本人！", i)
    }

    run("tags innate").unwrap();

    run(&format!("e -f tag1 test1 -t tag2 | echo \"{}\"", msg(1))).unwrap();
    run("-f innate -").expect_err("吃到了不該吃的標籤！");
    run("-f tag1 -").expect_err("吃到了不該吃的標籤！");
    assert_eq!(msg(1), run("-f tag2 -").unwrap());

    run(&format!("e -f tag1 test2 -t +tag2 | echo \"{}\"", msg(2))).unwrap();
    run("-f innate -").expect_err("吃到了不該吃的標籤！");
    assert_eq!(msg(2), run("-f tag1 -").unwrap());
    assert_eq!(msg(2), run("-f tag2 -").unwrap());

    run(&format!("e -f +tag1 test3 -t +tag2 | echo \"{}\"", msg(3))).unwrap();
    assert_eq!(msg(3), run("-f innate -").unwrap());
    assert_eq!(msg(3), run("-f tag1 -").unwrap());
    assert_eq!(msg(3), run("-f tag2 -").unwrap());
}

#[test]
fn test_multi_filter() {
    let _g = setup();
    run(&format!("e nobody | echo \"{}\"", MSG)).unwrap();
    run(&format!("-f test,pin e test-pin | echo \"{}\"", MSG)).unwrap();
    run(&format!("e -t pin pin-only | echo \"{}\"", MSG)).unwrap();

    assert_eq!(MSG, run("pin-only").unwrap());
    assert_eq!(MSG, run("test-pin").unwrap());
    assert_eq!(MSG, run("nobody").unwrap());

    run("tags +hidden").unwrap();
    assert_eq!(MSG, run("pin-only").unwrap());
    assert_eq!(MSG, run("test-pin").unwrap());
    run("nobody").expect_err("未能被主篩選器篩掉");

    run("tags +^test").unwrap();
    assert_eq!(MSG, run("pin-only").unwrap());
    run("test-pin").expect_err("未能被主篩選器篩掉");

    assert_eq!(MSG, run("-a test-pin").unwrap());
}

#[test]
fn test_rm() {
    let _g = setup();
    run("e longlive | echo 矻立不搖").unwrap();

    run(&format!("e test/ya -t test-tag | echo \"{}\"", MSG)).unwrap();
    assert_eq!(MSG, run("test/ya").unwrap());
    run("e . | echo \"你匿\"").unwrap();
    assert_eq!("你匿", run(".1").unwrap());

    run("rm - test*").unwrap();
    run("test/ya").expect_err("未能被刪除掉");
    run(".1").expect_err("未能被刪除掉");
    run("-a test/ya").expect_err("被刪除掉的腳本竟能用 `-a` 找回來");
    assert_eq!(MSG, run("-f removed test").unwrap());
    assert_eq!(
        MSG,
        run("-f test-tag test").expect("刪除沒有保留本來的標籤？")
    );

    assert_eq!(
        "你匿",
        run("-f removed,^test-tag -").expect("就算是匿名腳本也不該真的被刪掉！")
    );

    assert_eq!("矻立不搖", run("longlive").unwrap());

    run("e my/namespace/super-test | echo \"不要刪我 QmmmmQ\"").unwrap();
    assert_eq!("不要刪我 QmmmmQ", run("mysuper-test").unwrap());
    run("rm mysupertest").expect("刪除被命名空間搞爛了");
    run("mysuper-test").expect_err("未能被刪除掉");
    assert_eq!(
        "不要刪我 QmmmmQ",
        run("-f removed my/namespace/super-test").unwrap()
    );
    let file_path = run("-f removed which -").unwrap();
    let re = Regex::new(r".+my/namespace/\d{14}-super-test\.sh$").unwrap();
    assert!(re.is_match(&file_path), "路徑被刪除改爛：{}", file_path);

    assert_eq!("矻立不搖", run("longlive").unwrap());

    assert!(check_exist(&["longlive.sh"]));
    run("purge * -f all").expect("未能消滅掉一切（順便測了別名）");

    assert!(!check_exist(&["longlive.sh"]));
    run("-f all which").expect_err("沒有確實消滅掉一切");
}

#[test]
fn test_namespace_reorder_search() {
    let _g = setup();
    run(&format!(
        "e my/super/long/namespace-d/test-script | echo \"{}\"",
        MSG
    ))
    .unwrap();
    run(&format!(
        "e a/shorter/script -c js | console.log(\"{}\")",
        MSG_JS
    ))
    .unwrap();
    assert_eq!(MSG, run("myscript").expect("正常空間搜尋失敗"));
    assert_eq!(MSG, run("scriptsuper").expect("重排命名空間搜尋失敗"));
    assert_eq!(MSG, run("testlong").expect("重排命名空間搜尋失敗"));
    assert_eq!(MSG_JS, run("scrishorter").expect("重排命名空間搜尋失敗"));
    assert_eq!(MSG, run("namsplongsuery").expect("重排命名空間搜尋失敗"));
    run("script-test").expect_err("重排到腳本名字去了= =");
}

#[test]
fn test_append_tags() {
    let _g = setup();
    run("tags global").unwrap();
    run(&format!("-f +append e append-test | echo 附加標籤")).unwrap();
    run("-f no-append e no-append-test | echo 不要給我打標籤").unwrap();

    assert_eq!("附加標籤", run("apptest").unwrap());
    run("no-appendtest").expect_err("標籤還是附加上去了？");

    assert_eq!(
        "附加標籤",
        run("-f append apptest").expect("標籤沒附加上去？")
    );
    assert_eq!("不要給我打標籤", run("-f no-append apptest").unwrap());

    run("-f no-append mv no-append-test -t +eventually-append").unwrap();
    assert_eq!(
        "不要給我打標籤",
        run("-f eventually-append apptest").expect("標籤沒被 mv 附加上去？")
    );
    assert_eq!(
        "不要給我打標籤",
        run("-f no-append apptest").expect("標籤被 mv 弄壞了？")
    );
}

#[test]
fn test_miss_event() {
    let _g = setup();
    run(&format!("-f hide e hidden_first | echo 第一")).unwrap();
    run(&format!("-f hide e hidden_second | echo 第二")).unwrap();
    run(&format!("-f hide e third | echo 第三")).unwrap();
    assert_eq!("第三", run("!").unwrap());
    run("first").expect_err("執行了隱藏的腳本？？");
    assert_eq!("第一", run("!").unwrap(), "沒有記錄到錯過事件？");
}
