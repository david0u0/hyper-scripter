#[allow(dead_code)]
#[path = "tool.rs"]
mod tool;

use tool::*;

#[test]
fn test_history_args() {
    let _g = setup();
    run("e arg-receiver | # do nothing").unwrap();

    run(r#" receiver arg1 arg"2" arg\3 "#).unwrap();
    let recorded = run("history receiver").unwrap();
    let recorded = recorded.split("\n").next().unwrap().trim();

    assert_eq!(recorded, r#"arg1 "arg\"2\"" "arg\\3""#);
}
