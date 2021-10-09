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
    run!("e arg-receiver | # do nothing").unwrap();

    run!("{}", r#" receiver arg1 arg"2" arg\3 "#).unwrap();
    let recorded = run!("history show receiver").unwrap();
    assert_list(&recorded, &[r#"arg1 "arg\"2\"" "arg\\3""#]);
}

#[test]
fn test_no_trace() {
    let _g = setup();
    // TODO
}

#[test]
fn test_history_args_order() {
    let _g = setup();
    run!("e arg-receiver | # do nothing").unwrap();

    run!("receiver third").unwrap();
    run!("receiver second").unwrap();
    run!("receiver first").unwrap();

    let recorded = run!("history show receiver").unwrap();
    assert_list(&recorded, &["first", "second", "third"]);
    let recorded = run!("history show receiver --offset 1 --limit 1").unwrap();
    assert_list(&recorded, &["second"]);
    let recorded = run!("history show receiver --offset 2 --limit 999").unwrap();
    assert_list(&recorded, &["third"]);

    run!("receiver second").unwrap();
    run!("receiver third").unwrap();

    let recorded = run!("history show receiver").unwrap();
    assert_list(&recorded, &["third", "second", "first"]);
    let recorded = run!("history show receiver --offset 1 --limit 1").unwrap();
    assert_list(&recorded, &["second"]);
    let recorded = run!("history show receiver --offset 2 --limit 999").unwrap();
    assert_list(&recorded, &["first"]);
}

#[test]
fn test_history_args_rm() {
    let _g = setup();
    run!("e arg-receiver | echo $@").unwrap();

    run!("receiver third").unwrap();
    run!("receiver second").unwrap();
    run!("receiver first").unwrap();
    run!("receiver third").unwrap();
    run!("receiver second").unwrap();
    run!("receiver first").unwrap();
    run!("receiver third").unwrap();
    run!("receiver second").unwrap();
    run!("receiver first").unwrap();

    let recorded = run!("history show receiver").unwrap();
    assert_list(&recorded, &["first", "second", "third"]);
    let recorded = run!("history show receiver --offset 1 --limit 1").unwrap();
    assert_list(&recorded, &["second"]);
    let recorded = run!("history show receiver --offset 2 --limit 999").unwrap();
    assert_list(&recorded, &["third"]);

    run!("history rm receiver 0").expect_err("編號從1開始 =_=");

    run!("history rm receiver 1").unwrap(); // 幹掉 "first"

    let recorded = run!("history show receiver").unwrap();
    assert_list(&recorded, &["second", "third"]);

    assert_eq!(run!("run -l").unwrap(), "second", "沒有刪成功？");
    assert_eq!(
        run!("run -l - trailing").unwrap(),
        "second trailing",
        "沒有把參數往後接？"
    );

    run!("history rm receiver 2").unwrap(); // 幹掉 "second"
    let recorded = run!("history show receiver").unwrap();
    assert_list(&recorded, &["second trailing", "third"]);
}

#[test]
fn test_history_args_rm_last() {
    let _g = setup();

    run!("e A | echo A$@").unwrap();
    run!("e B | echo B$@").unwrap();
    run!("e C | echo C$@").unwrap();

    // 來點趣味性=_=
    let mut rng = rand::thread_rng();
    let mut maybe_dummy = move |script: &str, arg: &str| {
        use rand::Rng;
        let dummy = rng.gen::<u32>() % 3 == 0;
        let dummy_str = if dummy { "--dummy" } else { "" };
        let res = run!("run {} {} {} ", dummy_str, script, arg).unwrap();
        if !dummy {
            assert_eq!(res, format!("{}{}", script, arg));
        } else {
            assert_eq!(res, "");
        }
    };

    maybe_dummy("B", "x");
    run!("cat A").unwrap(); // read !
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

    run!("history rm A 2").unwrap(); // Ax

    assert_eq!(run!("run -l -").unwrap(), "Bzz");
    run!("history rm - 1").unwrap(); // Bzz
    run!("history rm - 1").unwrap(); // Bz
    assert_eq!(run!("run -l").unwrap(), "Az");
    run!("history rm - 1").unwrap(); // Az
    assert_eq!(run!("run -l -").unwrap(), "By");

    // Make some noise HAHA
    {
        maybe_dummy("B", "w");
        maybe_dummy("A", "w");

        assert_eq!(run!("run -l -").unwrap(), "Aw");
        run!("history rm B 1").unwrap(); // Bw
        assert_eq!(run!("run -l -").unwrap(), "Aw");
        run!("history rm A 1").unwrap(); // Aw
    }

    assert_eq!(run!("run -l -").unwrap(), "By"); // Ax already removed
    run!("history rm - 1").unwrap(); // By
    assert_eq!(run!("run -l -").unwrap(), "Ay");
    run!("history rm - 1").unwrap(); // Ay
    run!("run -l A").expect_err("previous args exist !?"); // fail, won't affect ordering

    assert_eq!(run!("run -l B").unwrap(), "Bx");
    run!("history rm - 1").unwrap(); // Bx
    run!("run -l B").expect_err("previous args exist !?");

    assert_eq!(run!("run -").unwrap(), "A"); // read time
    assert_eq!(run!("run ^^").unwrap(), "C"); // create time
}

#[test]
fn test_neglect_archaeology() {
    let _g = setup();
    let t1 = ScriptTest::new("1", None);
    let t2 = ScriptTest::new("2", None);
    let neg1 = ScriptTest::new("neg1", None);
    let neg2 = ScriptTest::new("neg2", None);
    t1.can_find_by_name().unwrap();
    t2.can_find_by_name().unwrap();
    neg1.can_find_by_name().unwrap();
    neg2.can_find_by_name().unwrap();

    run!("history neglect {}", neg1.get_name()).unwrap();
    run!("history neglect {}", neg2.get_name()).unwrap();

    t1.can_find_by_name().unwrap();
    t2.can_find_by_name().unwrap();
    t1.archaeology()
        .can_find_by_name()
        .expect_err("考古找到太新的腳本");
    t2.archaeology()
        .can_find_by_name()
        .expect_err("考古找到太新的腳本");

    neg1.can_find_by_name().unwrap_err();
    neg2.can_find_by_name().unwrap_err();
    neg1.archaeology()
        .can_find_by_name()
        .expect("考古找到不到舊腳本");
    neg2.archaeology()
        .can_find_by_name()
        .expect("考古找到不到舊腳本");

    run!("cat ={}!", neg1.get_name()).unwrap();
    neg1.can_find_by_name()
        .expect_err("讀取事件破壞了忽視的狀態");
    neg1.archaeology().can_find_by_name().unwrap();

    run!("mv ={}!", neg1.get_name()).unwrap();
    neg1.can_find_by_name().expect("移動事件沒有解除忽視狀態");
    neg1.archaeology().can_find_by_name().unwrap_err();

    neg2.can_find_by_name().unwrap_err();
    run!(dir: "aa", "={}!", neg2.get_name()).unwrap();
    neg2.can_find_by_name().expect("執行事件沒有解除忽視狀態");
    neg2.archaeology().can_find_by_name().unwrap_err();
}
