#[allow(dead_code)]
#[path = "tool.rs"]
mod tool;

use tool::*;

fn assert_list(actual: &str, expected: &[&str]) {
    let actual_v: Vec<_> = actual
        .split("\n")
        .filter_map(|s| if s.len() > 0 { Some(s.trim()) } else { None })
        .collect();
    assert_eq!(expected, actual_v);
}

#[test]
fn test_history_args() {
    let _g = setup();
    run("e arg-receiver | # do nothing").unwrap();

    run(r#" receiver arg1 arg"2" arg\3 "#).unwrap();
    let recorded = run("history show receiver").unwrap();
    assert_list(&recorded, &[r#"arg1 "arg\"2\"" "arg\\3""#]);
}

#[test]
fn test_history_args_order() {
    let _g = setup();
    run("e arg-receiver | # do nothing").unwrap();

    run("receiver third").unwrap();
    run("receiver second").unwrap();
    run("receiver first").unwrap();

    let recorded = run("history show receiver").unwrap();
    assert_list(&recorded, &["first", "second", "third"]);
    let recorded = run("history show receiver --offset 1 --limit 1").unwrap();
    assert_list(&recorded, &["second"]);
    let recorded = run("history show receiver --offset 2 --limit 999").unwrap();
    assert_list(&recorded, &["third"]);

    run("receiver second").unwrap();
    run("receiver third").unwrap();

    let recorded = run("history show receiver").unwrap();
    assert_list(&recorded, &["third", "second", "first"]);
    let recorded = run("history show receiver --offset 1 --limit 1").unwrap();
    assert_list(&recorded, &["second"]);
    let recorded = run("history show receiver --offset 2 --limit 999").unwrap();
    assert_list(&recorded, &["first"]);
}

#[test]
fn test_history_args_rm() {
    let _g = setup();
    run("e arg-receiver | echo $@").unwrap();

    run("receiver third").unwrap();
    run("receiver second").unwrap();
    run("receiver first").unwrap();
    run("receiver third").unwrap();
    run("receiver second").unwrap();
    run("receiver first").unwrap();
    run("receiver third").unwrap();
    run("receiver second").unwrap();
    run("receiver first").unwrap();

    let recorded = run("history show receiver").unwrap();
    assert_list(&recorded, &["first", "second", "third"]);
    let recorded = run("history show receiver --offset 1 --limit 1").unwrap();
    assert_list(&recorded, &["second"]);
    let recorded = run("history show receiver --offset 2 --limit 999").unwrap();
    assert_list(&recorded, &["third"]);

    run("history rm receiver 1").unwrap(); // 幹掉 "first"

    let recorded = run("history show receiver").unwrap();
    assert_list(&recorded, &["second", "third"]);

    assert_eq!(run("run -p -").unwrap(), "second", "沒有刪成功？");

    run("history rm receiver 2").unwrap(); // 幹掉 "third"
    let recorded = run("history show receiver").unwrap();
    assert_list(&recorded, &["second"]);
}

#[test]
fn test_history_args_rm_last() {
    let _g = setup();

    run("e A | echo A$@").unwrap();
    run("e B | echo B$@").unwrap();
    run("e C | echo C$@").unwrap();

    run("B x").unwrap();
    run("cat A").unwrap(); // read !
    run("A x").unwrap(); // removed later
    run("A y").unwrap();
    run("B y").unwrap();
    run("A x").unwrap(); // removed later
    run("A z").unwrap(); // overwrittern
    run("B z").unwrap(); // overwrittern
    run("A z").unwrap();
    run("B zz").unwrap(); // overwrittern
    run("B z").unwrap();
    run("B zz").unwrap();

    run("history rm A 2").unwrap(); // x

    assert_eq!(run("run -p -").unwrap(), "Bzz");
    run("history rm - 1").unwrap(); // Bzz
    run("history rm - 1").unwrap(); // Bz
    assert_eq!(run("run -p -").unwrap(), "Az");
    run("history rm - 1").unwrap(); // Az
    assert_eq!(run("run -p -").unwrap(), "By");

    // Make some noise HAHA
    {
        assert_eq!(run("run B w").unwrap(), "Bw");
        assert_eq!(run("run A w").unwrap(), "Aw");

        assert_eq!(run("run -p -").unwrap(), "Aw");
        run("history rm B 1").unwrap(); // Bw
        assert_eq!(run("run -p -").unwrap(), "Aw");
        run("history rm A 1").unwrap(); // Aw
    }

    assert_eq!(run("run -p -").unwrap(), "By"); // Ax already removed
    run("history rm - 1").unwrap(); // By
    assert_eq!(run("run -p -").unwrap(), "Ay");
    run("history rm - 1").unwrap(); // Ay
    run("run -p A").expect_err("previous args exist !?"); // fail, won't affect ordering

    assert_eq!(run("run -p B").unwrap(), "Bx");
    run("history rm - 1").unwrap(); // Bx
    run("run -p B").expect_err("previous args exist !?");

    assert_eq!(run("run -").unwrap(), "A"); // read time
    assert_eq!(run("run ^^").unwrap(), "C"); // create time
}
