#[allow(dead_code)]
#[path = "tool.rs"]
mod tool;

use tool::*;

use rand::Rng;

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
    // TODO
}

#[test]
fn test_humble_amend_rm_id() {
    let _g = setup();
    // 注意刪除後 humble 事件為最新的後果（可能因為寫回的緣故，越刪除時間反而更新）
    const CONTENT: &str = r#"
    if [ "$1" = "h" ]; then
        $HS_EXE -H $HS_HOME history humble $HS_RUN_ID
    elif [ "$1" = "r" ]; then
        $HS_EXE -H $HS_HOME history rm-id $HS_RUN_ID
    elif [ "$1" = "a" ]; then
        $HS_EXE -H $HS_HOME history amend $HS_RUN_ID do-the-amend
    fi"#;

    let test = ScriptTest::new("test", None, Some(CONTENT));
    let baseline = ScriptTest::new("baseline", None, None);

    fn assert_last(s: &ScriptTest) {
        s.can_find("-").unwrap();
    }
    let assert_history = |list: &[&str]| {
        let recorded = run!("history show {}", test.get_name()).unwrap();
        assert_list(&recorded, list);
    };

    test.filter("--humble").run("flag-humble").unwrap();
    assert_last(&baseline);
    test.run("normal").unwrap();
    assert_last(&test);
    run!("history rm {} 1", test.get_name()).unwrap();
    assert_last(&baseline); // `flag-humble` 為 humbe 事件，不會影響最新事件時間

    test.run("h").unwrap();
    assert_last(&baseline);
    assert_history(&["h", "flag-humble"]);

    test.run("r").unwrap();
    assert_last(&baseline);
    assert_history(&["h", "flag-humble"]);

    test.filter("--dummy").run("r").unwrap(); // 因為是 --dummy 所以不會觸發 rm-id
    assert_last(&test);
    baseline.run("").unwrap();
    test.run("r").unwrap();
    assert_last(&baseline);
    assert_history(&["r", "h", "flag-humble"]); // `r` 不會因為 rm-id 就被刪掉

    run!("history rm {} 1", baseline.get_name()).unwrap();

    test.run("normal").unwrap();
    assert_history(&["normal", "r", "h", "flag-humble"]);
    run!("history rm {} 1", test.get_name()).unwrap();
    assert_last(&test); // 刪了一個還有一個
    run!("history rm {} 1", test.get_name()).unwrap();
    assert_last(&baseline); // `h` 為 humble，不會影響最新事件時間
    assert_history(&["h", "flag-humble"]);

    test.run("a").unwrap();
    assert_last(&test);
    assert_history(&["do-the-amend", "h", "flag-humble"]);

    baseline.run("").unwrap();
    test.run("h").unwrap();
    assert_last(&baseline);
    assert_history(&["h", "do-the-amend", "flag-humble"]);

    // 測一下 --no-trace 會不會搞爛東西
    run!("history rm {} 1..", test.get_name()).unwrap();
    test.filter("--no-trace").run("a").unwrap();
    test.filter("--no-trace").run("r").unwrap();
    test.filter("--no-trace").run("h").unwrap();
    assert_history(&[]);
    // 測一下 --humble 會不會搞爛東西
    test.filter("--humble").run("a").unwrap();
    test.filter("--humble").run("r").unwrap();
    test.filter("--humble").run("h").unwrap();
    assert_history(&["h", "do-the-amend"]);
    assert_last(&baseline);
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

    assert_eq!(run!("run -p").unwrap(), "second", "沒有刪成功？");
    assert_eq!(
        run!("run -p - trailing").unwrap(),
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

    assert_eq!(run!("run -pE -").unwrap(), "Bzz");
    run!("history rm - 1").unwrap(); // Bzz
    run!("history rm - 1").unwrap(); // Bz
    assert_eq!(run!("run -pE").unwrap(), "Az");
    run!("history rm - 1").unwrap(); // Az
    assert_eq!(run!("run -pE -").unwrap(), "By");

    // Make some noise HAHA
    {
        maybe_dummy("B", "w");
        maybe_dummy("A", "w");

        assert_eq!(run!("run -pE -").unwrap(), "Aw");
        run!("history rm B 1").unwrap(); // Bw
        assert_eq!(run!("run -pE -").unwrap(), "Aw");
        run!("history rm A 1").unwrap(); // Aw
    }

    assert_eq!(run!("run -pE -").unwrap(), "By"); // Ax already removed
    run!("history rm - 1").unwrap(); // By
    assert_eq!(run!("run -pE -").unwrap(), "Ay");
    run!("history rm - 1").unwrap(); // Ay
    run!("run -pE A").expect_err("沒有先前參數");
    assert_eq!(run!("--no-trace -p A").expect("空的"), "A"); // no-trace, won't effect order

    assert_eq!(run!("run -pE B").unwrap(), "Bx");
    run!("history rm - 1").unwrap(); // Bx
    run!("run -pE B").expect_err("沒有先前參數");
    assert_eq!(run!("--no-trace -p B").expect("空的"), "B"); // no-trace, won't effect order

    assert_eq!(run!("run -").unwrap(), "A"); // read time
    assert_eq!(run!("run ^^").unwrap(), "C"); // create time
}

#[test]
fn test_neglect_archaeology() {
    let _g = setup();
    let t1 = ScriptTest::new("1", None, None);
    let t2 = ScriptTest::new("2", None, None);
    let neg1 = ScriptTest::new("neg1", None, None);
    let neg2 = ScriptTest::new("neg2", None, None);
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
    run!("={}!", neg2.get_name()).unwrap();
    neg2.can_find_by_name().expect("執行事件沒有解除忽視狀態");
    neg2.archaeology().can_find_by_name().unwrap_err();
}

#[test]
fn test_event_path() {
    // TODO: 帶目錄的刪除？
    // TODO: soft link
    let _g = setup();
    let tmp_dir = std::env::temp_dir();
    let init_dir = |s: &str| -> (String, String) {
        let p = tmp_dir.join(s);
        std::fs::create_dir_all(&p).unwrap();
        (p.to_string_lossy().to_string(), s.to_owned())
    };
    let (dir_a, rel_a) = init_dir("a");
    let (dir_b, rel_b) = init_dir("b");
    let (dir_c, _) = init_dir("c");

    run!("e . | echo $1").unwrap();
    run!(dir: &dir_a, "- a").unwrap();
    run!(dir: &dir_b, "- b").unwrap();
    run!(dir: &dir_c, "- c").unwrap();
    run!(dir: &dir_a, "- c").unwrap();

    const SHOW: &str = "history show -";
    let do_test = move || {
        let recorded = run!("{}", SHOW).unwrap();
        assert_list(&recorded, &["c", "b", "a"]);

        let recorded = run!("{} --dir {}", SHOW, dir_a).unwrap();
        assert_list(&recorded, &["c", "a"]);

        let recorded = run!("{} --dir {}", SHOW, dir_c).unwrap();
        assert_list(&recorded, &["c"]);

        let recorded = run!("{} --dir {}/test/../../{}", SHOW, dir_b, rel_b).unwrap();
        assert_list(&recorded, &["b"]);

        let recorded =
            run!(dir: &tmp_dir, "{} --dir gg/../bb/../{}", SHOW, rel_a).expect("相對路徑就壞了？");
        assert_list(&recorded, &["c", "a"]);

        // NOTE: 沒有 --no-trace 的話，下一次執行的順序會跑掉
        let output =
            run!(dir: &tmp_dir, "--no-trace run -p --dir {}", dir_b).expect("執行前一次參數壞了？");
        assert_eq!(output, "b");

        let recorded = run!("{} --dir a/b/c/d", SHOW).expect("路徑不存在就壞了？");
        assert_list(&recorded, &[]);
    };

    do_test();
    run!("history tidy -").unwrap();
    do_test();
}

#[test]
fn test_humble_and_time_filter() {
    let _g = setup();
    const CONTENT: &str = r#"
    $HS_EXE -H $HS_HOME history humble $HS_RUN_ID
    "#;
    let test = ScriptTest::new("test", None, Some(CONTENT));
    run!("history neglect {}", test.get_name()).unwrap();
    test.can_find_by_name().unwrap_err();

    test.filter("-a --dummy").run("a").unwrap(); // normal
    test.can_find_by_name().unwrap();
    run!("history rm - 1").unwrap();
    test.can_find_by_name().unwrap_err();

    test.filter("-a").run("b").unwrap(); // humble by id
    test.can_find_by_name().unwrap();
    run!("history rm - 1").unwrap();
    test.can_find_by_name().unwrap_err();

    test.filter("--humble -a").run("c").unwrap(); // humble by flag
    test.can_find_by_name().unwrap();
    run!("history rm - 1").unwrap();
    test.can_find_by_name().unwrap_err();

    run!("--humble cat ={}!", test.get_name()).unwrap();
    test.can_find_by_name()
        .expect_err("謙卑讀取事件不應影響時間篩選");
}

#[test]
fn test_multi_history() {
    println!("多腳本歷史測試");
    let _g = setup();

    use std::collections::{HashMap, HashSet};
    struct Historian {
        counter: u32,
        m: HashMap<(String, u32), u32>,
    }
    macro_rules! run_n_print {
        ($($arg:tt)*) => ({
            let cmd = format!($($arg)*);
            println!("{}", cmd);
            run!(silent: true, "{}", cmd).unwrap()
        });
    }
    impl Historian {
        fn new() -> Self {
            Historian {
                counter: 0,
                m: Default::default(),
            }
        }
        fn get_show_list(&self) -> Vec<(String, u32)> {
            let mut v: Vec<_> = self.m.iter().collect();
            v.sort_by_key(|(_, &v)| -(v as i32));
            v.into_iter()
                .map(|((name, arg), _)| (name.to_owned(), *arg))
                .collect()
        }
        fn get_order(&self) -> Vec<String> {
            let mut s: HashSet<_> = HashSet::new();
            let mut ret = vec![];
            let args = self.get_show_list();
            for (name, _) in args.iter() {
                let absent = s.insert(name);
                if absent {
                    ret.push(name.to_owned());
                }
            }
            ret
        }
        fn len(&self) -> usize {
            self.m.len()
        }
        fn run(&mut self, script: &ScriptTest, arg: u32) {
            let name = script.get_name().to_owned();
            run_n_print!("run --dummy {}! {}", name, arg);

            self.counter += 1;
            self.m.insert((name, arg), self.counter);
        }
        fn rm(&mut self, min: usize, max: usize) {
            run_n_print!("history rm * {}..{}", min + 1, max + 1);

            let list = self.get_show_list();
            for line in &list[min..max] {
                self.m.remove(line);
            }
        }
        fn show(&self) {
            let expected: Vec<_> = self
                .get_show_list()
                .iter()
                .map(|(name, arg)| format!("{} {}", name, arg))
                .collect();
            let actual: Vec<_> = run_n_print!("-a history show * --limit 999 --with-name")
                .lines()
                .map(|s| s.to_owned())
                .collect();
            assert_eq!(expected, actual);
        }
        fn ls(&self) {
            let expected = self.get_order();
            let actual: Vec<_> = run_n_print!("ls --grouping none --name --plain")
                .split_whitespace()
                .map(|s| s.to_owned())
                .collect();
            assert_eq!(expected, actual);
        }
    }

    let mut h = Historian::new();
    fn new_script(name: &str) -> ScriptTest {
        let s = ScriptTest::new(name, None, None);
        run!("history neglect {}", name).unwrap();
        s
    }
    let a = new_script("a");
    let b = new_script("b");
    let c = new_script("c");
    let d = new_script("d");
    let repo = [a, b, c, d];
    let mut rng = rand::thread_rng();

    for _ in 0..150 {
        macro_rules! run_script {
            () => {
                let script_idx: usize = rng.gen_range(0..4);
                let arg = rng.gen_range(0..5);
                h.run(&repo[script_idx], arg);
            };
        }

        let len = h.len();
        if len == 0 {
            run_script!();
            continue;
        }

        let action: u32 = rng.gen_range(0..3);
        match action {
            1 => {
                // rm
                loop {
                    let t1 = rng.gen_range(0..len);
                    let t2 = rng.gen_range(t1 + 1..len + 1);
                    h.rm(t1, t2);
                    break;
                }
            }
            _ => {
                // run
                run_script!();
            }
        }
        h.show();
        h.ls();
    }
}
