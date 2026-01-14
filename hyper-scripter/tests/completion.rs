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
    let home = get_home();
    let completion_file = format!("{}/../completion/hs.fish", env!("CARGO_MANIFEST_DIR"));
    let fish_cmd = format!(
        "source {} && complete -C 'hs -H {} {}'",
        completion_file,
        home.to_string_lossy(),
        cmd
    );
    run_cmd(
        "fish",
        &["--no-config", "-c", &fish_cmd],
        Default::default(),
    )
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
    assert_eq!("test/1\ttest/1", run_completion("1test").unwrap());
    assert_eq!("test/3\ttest/3", run_completion("3test").unwrap());
}

#[test]
fn test_time_order() {
    setup_scripts();

    assert_eq!(
        "test/3\t^1\ntest/1\t^2\nnot/related\t^3",
        run_completion("ls ").unwrap()
    );
    assert_eq!(
        "test/3\ttest/3\ntest/1\ttest/1",
        run_completion("tes").unwrap()
    );

    run!("cat test/1").unwrap();

    assert_eq!(
        "test/1\t^1\ntest/3\t^2\nnot/related\t^3",
        run_completion("ls ").unwrap()
    );
    assert_eq!(
        "test/1\ttest/1\ntest/3\ttest/3",
        run_completion("tes").unwrap()
    );
}

#[test]
fn test_bang() {
    setup_scripts();
    const PARTIAL: &str = "test/3\ttest/3\ntest/1\ttest/1";
    const ALL: &str = "test/3!\ttest/3!\ntest/2!\ttest/2!\ntest/1!\ttest/1!";

    assert_eq!(PARTIAL, run_completion("tes").unwrap());
    assert_eq!(ALL, run_completion("tes!").unwrap());

    assert_eq!(PARTIAL, run_completion("ls tes").unwrap());
    assert_eq!(ALL, run_completion("ls tes!").unwrap());

    // test when alias
    assert_eq!(PARTIAL, run_completion("e tes").unwrap());
    assert_eq!(ALL, run_completion("e tes!").unwrap());

    // test when complex alias
    run!("alias c cat --with 'bat --paging=always'").unwrap();
    assert_eq!(PARTIAL, run_completion("c tes").unwrap());
    assert_eq!(ALL, run_completion("c tes!").unwrap());
}

#[test]
fn test_tags_and_types() {
    setup_scripts();

    const ALL_TAGS: &str = "+new-tag\t+new-tag\n+hide\t+hide\n+remove\t+remove\n+all\t+all";

    assert_eq!(
        split_and_sort("pin\nno-hidden\nno-removed"),
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
