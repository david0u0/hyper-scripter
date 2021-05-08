#[allow(dead_code)]
#[path = "tool.rs"]
mod tool;

use tool::*;

fn assert_list(actual: &str, expected: &[&str]) {
    let actual_v: Vec<_> = actual
        .split('\n')
        .filter_map(|s| if !s.is_empty() { Some(s.trim()) } else { None })
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

    run("history rm receiver 0").expect_err("編號從1開始 =_=");

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

    // 來點趣味性=_=
    let mut rng = rand::thread_rng();
    let mut maybe_dummy = move |script: &str, arg: &str| {
        use rand::Rng;
        let dummy = rng.gen::<u32>() % 3 == 0;
        let dummy_str = if dummy { "--dummy" } else { "" };
        let res = run(format!("run {} {} {} ", dummy_str, script, arg)).unwrap();
        if !dummy {
            assert_eq!(res, format!("{}{}", script, arg));
        } else {
            assert_eq!(res, "");
        }
    };

    maybe_dummy("B", "x");
    run("cat A").unwrap(); // read !
    maybe_dummy("A", "x"); // removed later
    maybe_dummy("A", "y");
    maybe_dummy("B", "y");
    maybe_dummy("A", "x"); // removed later
    maybe_dummy("A", "z"); // overwrittern
    maybe_dummy("B", "z"); // overwrittern
    maybe_dummy("A", "z");
    maybe_dummy("B", "zz"); // overwrittern
    maybe_dummy("B", "z");
    maybe_dummy("B", "zz");

    run("history rm A 2").unwrap(); // Ax

    assert_eq!(run("run -p -").unwrap(), "Bzz");
    run("history rm - 1").unwrap(); // Bzz
    run("history rm - 1").unwrap(); // Bz
    assert_eq!(run("run -p").unwrap(), "Az");
    run("history rm - 1").unwrap(); // Az
    assert_eq!(run("run -p -").unwrap(), "By");

    // Make some noise HAHA
    {
        maybe_dummy("B", "w");
        maybe_dummy("A", "w");

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
