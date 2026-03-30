#[allow(dead_code)]
#[path = "tool.rs"]
mod tool;
use tool::*;

pub use hyper_scripter::path::get_home;

fn split_and_sort(s: &str) -> Vec<String> {
    let mut s: Vec<_> = s.split('\n').map(|s| s.to_owned()).collect();
    s.sort();
    s
}

fn run_completion(cmd: &str) -> Result<String, Error> {
    let trailing = if cmd.ends_with(" ") { "''" } else { "" };
    run!(completion: true, "{cmd}{trailing}")
}

fn setup_scripts() {
    setup();
    run!("e not/related").unwrap();
    run!("e test/1").unwrap();
    run!("e -t hide test/2").unwrap();
    run!("e -t new-tag test/3").unwrap();
}

#[test]
fn test_reorder_name() {
    setup_scripts();
    assert_eq!("test/1\t1test", run_completion("1test").unwrap());
    assert_eq!("test/3\t3test", run_completion("3test").unwrap());
}

#[test]
fn test_time_order() {
    setup_scripts();

    assert_eq!(
        "test/3\t^1\ntest/1\t^2\nnot/related\t^3",
        run_completion("ls ").unwrap()
    );
    assert_eq!(
        "test/3\ttest\ntest/1\ttest",
        run_completion("test").unwrap()
    );

    run!("cat test/1").unwrap();

    assert_eq!(
        "test/1\t^1\ntest/3\t^2\nnot/related\t^3",
        run_completion("ls ").unwrap()
    );
    assert_eq!(
        "test/1\ttest\ntest/3\ttest",
        run_completion("test").unwrap()
    );
}

#[test]
fn test_bang() {
    setup_scripts();
    const PARTIAL: &str = "test/3\ttest\ntest/1\ttest";
    const ALL: &str = "test/3!\ttest!\ntest/2!\ttest!\ntest/1!\ttest!";

    assert_eq!(PARTIAL, run_completion("test").unwrap());
    assert_eq!(ALL, run_completion("test!").unwrap());

    assert_eq!(PARTIAL, run_completion("ls test").unwrap());
    assert_eq!(ALL, run_completion("ls test!").unwrap());

    // test when alias
    assert_eq!(PARTIAL, run_completion("e test").unwrap());
    assert_eq!(ALL, run_completion("e test!").unwrap());

    // test when complex alias
    run!("alias c cat --with 'bat --paging=always'").unwrap();
    assert_eq!(PARTIAL, run_completion("c test").unwrap());
    assert_eq!(ALL, run_completion("c test!").unwrap());
}

#[test]
fn test_tags_and_types() {
    setup_scripts();

    const ALL_TAGS: &str = "+new-tag\ttags\n+hide\ttags\n+remove\ttags\n+all\ttags";

    assert_eq!(
        split_and_sort("pin\t+pin,util\nno-hidden\t+^hide!\nno-removed\t+^remove!"),
        split_and_sort(&run_completion("t set --name ").unwrap())
    );
    assert_eq!(
        split_and_sort(ALL_TAGS),
        split_and_sort(&run_completion("t set ").unwrap())
    );

    // TODO: also test the for types
    // assert_eq!(ALL_TAGS, run_completion("s ls -s ").unwrap());
    // assert_eq!(ALL_TAGS, run_completion("s e -s ").unwrap());
}
