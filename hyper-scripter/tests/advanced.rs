#[allow(dead_code)]
#[path = "tool.rs"]
mod tool;
pub use tool::*;

use hyper_scripter::{
    script_type::ScriptFullType,
    util::{remove, write_file},
};
use std::path::PathBuf;

#[test]
fn test_mv_dir() {
    let _g = setup();
    ScriptTest::new("file1", Some("remove"), None);
    ScriptTest::new("dir1/file1", None, None);
    ScriptTest::new("dir1/dir2/file1", None, None);
    ScriptTest::new("dir3/native", None, None);
    ScriptTest::new("dirless", None, None);

    run!("mv dir1/* dir3/ -t hide").expect_err("做為移動目標的兩個 file1 撞名");

    run!("mv d1d2 dir1/dir2/file2").unwrap();
    run!("mv dir1/* dir3/ -t hide").expect("不再撞名");
    assert_ls(vec!["dir3/native", "dirless"], None, None);
    run!("tags all,^remove").unwrap();
    assert_ls(
        vec!["dir3/file1", "dir3/file2", "dir3/native", "dirless"],
        None,
        None,
    );

    run!("mv dir3/file* /").expect_err("移動目標與既存的 file1 撞名");

    run!("mv file1! file4").unwrap();
    run!("mv dir3/file* /").expect("不再撞名");
    assert_ls(
        vec!["file1", "file2", "file4", "dir3/native", "dirless"],
        Some("all"),
        None,
    );
}

// TODO: test cp

#[test]
fn test_sub_tmpl() {
    let _g = setup();
    fn get_types_vec() -> Vec<String> {
        let mut v: Vec<_> = run!("types ls")
            .unwrap()
            .split_ascii_whitespace()
            .map(|s| s.to_owned())
            .collect();
        v.sort();
        v
    }
    fn modify_types_vec(og: &mut Vec<String>, new: &[&str], rmed: &[&str]) {
        og.retain(|s| !rmed.contains(&s.as_ref()));
        og.extend(new.iter().map(|s| s.to_string()));
        og.sort();
    }
    fn get_tmpl_path(name: &str) -> PathBuf {
        let ty: ScriptFullType = name.parse().unwrap();
        hyper_scripter::path::get_template_path(&ty).unwrap()
    }

    const RB_TRAVERSE: &str = "rb/traverse";
    const JS_SUPER_WEIRD_NAME: &str = "js/suPeR-weIRd_NAme";
    const WEIRD_JS_STR: &str = "this is a super weird JS string";

    run!("e traverse-test -T {} | puts 'test!'", RB_TRAVERSE).unwrap();

    let mut types = get_types_vec();
    let p = get_tmpl_path(RB_TRAVERSE);
    remove(&p).unwrap();
    run!("e ? -T {} | puts 'test!'", RB_TRAVERSE).expect_err("子模版被砍了就不該創新的");

    assert_ne!(get_types_vec(), types); // rb/traverse 還在向量中
    modify_types_vec(&mut types, &[], &[RB_TRAVERSE]);
    assert_eq!(get_types_vec(), types);

    assert_eq!(
        run!("types js").unwrap(),
        run!("types {}", JS_SUPER_WEIRD_NAME).unwrap(),
        "子模版的預設值應該和父類別相同（除非是寫死的那幾個，如 rb/traverse）"
    );
    let p = get_tmpl_path(JS_SUPER_WEIRD_NAME);
    write_file(&p, &format!("console.log('{WEIRD_JS_STR}')")).unwrap();
    assert_ne!(
        run!("types js").unwrap(),
        run!("types {}", JS_SUPER_WEIRD_NAME).unwrap(),
        "子模版檔案已被寫入，不該相同"
    );

    assert_ne!(get_types_vec(), types); // 怪名字還不在向量中
    modify_types_vec(&mut types, &[JS_SUPER_WEIRD_NAME], &[]);
    assert_eq!(get_types_vec(), types);

    run!("e weird-test -T {} -f", JS_SUPER_WEIRD_NAME).unwrap();
    assert_eq!(WEIRD_JS_STR, run!("weird-test").unwrap());
    run!("traverse-test").expect("刪個子模版不應影響已存在的腳本！");
}

#[test]
fn test_bang_list_query() {
    let _g = setup();
    let a = ScriptTest::new("dir/a", Some("hide"), None);
    let b = ScriptTest::new("dir/b", Some("hide"), None);
    let c = ScriptTest::new("dir/c", Some("hide"), None);
    let d = ScriptTest::new("dir/d", None, None);
    let e = ScriptTest::new("e", None, None);

    assert_ls(vec![&d], None, Some("dir/*"));
    assert_ls(vec![&d, &e], None, Some("*"));
    assert_ls(vec![&a, &b, &c, &d], None, Some("dir/*!"));
    run!("ls dir2/*!").expect_err("不存在的列表查詢還是得報錯");
}

#[test]
fn test_type_select() {
    let _g = setup();
    let a_rb = "a";
    let b = "b";
    let c_rb = "c";
    let d = "d";
    let e_hide_rb = "e";
    let f_tag_rb = "f";

    run!("e -f {}", b).unwrap();
    run!("e -f {}", d).unwrap();
    run!("e -f -T rb {}", a_rb).unwrap();
    run!("e -f -T rb {}", c_rb).unwrap();
    run!("e -f -t hide -T rb {}", e_hide_rb).unwrap();
    run!("e -f -t tag -T rb {}", f_tag_rb).unwrap();

    assert_ls(vec![a_rb, b, c_rb, d, f_tag_rb], None, None);
    assert_ls(vec![b, d], Some("@sh"), None);
    assert_ls(vec![b, d, f_tag_rb], Some("@sh,tag"), None);

    assert_ls(vec![a_rb, c_rb, f_tag_rb], Some("+^@sh"), None);
    assert_ls(vec![a_rb, c_rb], Some("+^@sh,^tag"), None);

    assert_ls(vec![a_rb, c_rb, e_hide_rb, f_tag_rb], Some("@rb"), None);
    assert_ls(vec![a_rb, b, c_rb, d, f_tag_rb], Some("+@rb"), None);

    assert_ls(vec![a_rb, c_rb, f_tag_rb], Some("+@rb!"), None);
}

#[test]
fn test_prev_env() {
    const MY_ENV: &str = "MY_ENV";
    const MY_OTHER_ENV: &str = "MY_OTHER_ENV";
    const MY_ENV_HELP: &str = "MY_ENV_HELP";
    run!(
        "e --no-template ? | 
        # [HS_ENV]: {}
        # [HS_ENV]: {}
        # [HS_ENV_HELP]: {}
        echo ${}:${}:${}
        ",
        MY_ENV,
        MY_OTHER_ENV,
        MY_ENV_HELP,
        MY_ENV,
        MY_OTHER_ENV,
        MY_ENV_HELP,
    )
    .unwrap();

    let env = vec![
        (MY_ENV.to_owned(), "A".to_owned()),
        (MY_ENV_HELP.to_owned(), "C".to_owned()),
    ];

    assert_eq!(run!("-").unwrap(), "::");
    assert_eq!(run!("run -p").unwrap(), "::");

    assert_eq!(run!(custom_env: env, "run -p").unwrap(), "A::C");
    assert_eq!(run!("run -p").unwrap(), "A::");
    assert_eq!(run!("run -p").unwrap(), "A::");
    assert_eq!(run!("run -p").unwrap(), "A::");

    let env = vec![
        (MY_ENV.to_owned(), "X".to_owned()),
        (MY_OTHER_ENV.to_owned(), "B".to_owned()),
    ];
    // -p is stronger than normal env var
    assert_eq!(run!(custom_env: env.clone(), "run -p").unwrap(), "A:B:");
    assert_eq!(run!("run -p").unwrap(), "A:B:");

    assert_eq!(run!(custom_env: env, "run -").unwrap(), "X:B:");
    assert_eq!(run!("run -p").unwrap(), "X:B:");

    assert_eq!(run!("-").unwrap(), "::");
    assert_eq!(run!("run -p").unwrap(), "::");
}
