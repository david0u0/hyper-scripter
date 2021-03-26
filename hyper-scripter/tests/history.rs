#[allow(dead_code)]
#[path = "tool.rs"]
mod tool;

use tool::*;

fn assert_list(actual: &str, expected: &[&str]) {
    let actual_v: Vec<_> = actual
        .split("\n")
        .skip(1)
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
