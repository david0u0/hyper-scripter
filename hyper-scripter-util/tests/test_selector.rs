use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

fn do_select_lines(seq: &str) -> Vec<String> {
    let seq = seq.replace(' ', "");
    let script_path = "tests/test_selector.rb";
    let mut cmd = Command::new("ruby");
    let mut child = cmd
        .args(vec![script_path, &seq])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let stdout = child.stdout.as_mut().unwrap();
    let mut out_str = Vec::<String>::new();
    let reader = BufReader::new(stdout);
    reader
        .lines()
        .filter_map(|line| line.ok())
        .for_each(|line| {
            println!("{}", line);
            out_str.push(line);
        });
    let status = child.wait().unwrap();
    if !status.success() {
        panic!("執行選擇器失敗：{}", status);
    };
    out_str
}
fn do_select(seq: &str) -> String {
    let res = do_select_lines(seq);
    res.join("\n")
}

#[test]
fn test_nevigation() {
    assert_eq!("12-g", do_select("k\r"));
    assert_eq!("2-b", do_select("j\r"));
    assert_eq!("4-c", do_select("jkkkjjjkkkkjjjjj\r"));

    assert_eq!(
        vec!["d", "10-f"],
        do_select_lines("5\r p jkkkjjjkkkkjjjjj\r")
    );
    assert_eq!("12-g", do_select("99\r\r"));

    assert_eq!(
        vec!["e", "e", "10-f", "8-e"],
        do_select_lines("/8-\r p np jjkA n\r")
    );
    assert_eq!(
        vec!["c", "12-g", "b", "12-g"],
        do_select_lines("4\rp /2-\rA np n\r")
    );
}

#[test]
fn test_range() {
    assert_eq!(
        vec![
            "4-c",
            "range print b",
            "range print c",
            "====",
            "range print c",
            "range print d",
            "range print e",
            "range print f",
            "====",
            "b",
            "c",
            "d",
        ],
        do_select_lines("jjA v 3\rp \rA\rAA l \rA\r kkkp \rA\r l vkkv /2\rnP")
    );
}

#[test]
fn test_deletion() {
    assert_eq!(
        vec![
            "delete b",
            "delete e",
            "range delete c",
            "range delete d",
            "range delete f",
            "a",
            "g"
        ],
        do_select_lines("jd jjd v /4\rd P")
    );
}
