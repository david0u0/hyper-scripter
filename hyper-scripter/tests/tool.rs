use std::fs::canonicalize;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::ExitStatus;
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard};

lazy_static::lazy_static! {
    static ref LOCK: Mutex<()> = Mutex::new(());
}
#[cfg(not(debug_assertions))]
const EXE: &'static str = "../target/release/hs";
#[cfg(debug_assertions)]
const EXE: &'static str = "../target/debug/hs";

pub fn get_exe_abs() -> String {
    canonicalize(EXE)
        .unwrap()
        .to_string_lossy()
        .as_ref()
        .to_owned()
}

const HOME: &str = "./.hyper_scripter";

pub fn get_home() -> PathBuf {
    canonicalize(HOME).unwrap()
}
pub fn setup<'a>() -> MutexGuard<'a, ()> {
    setup_home(get_home())
}
pub fn setup_home<'a>(home: PathBuf) -> MutexGuard<'a, ()> {
    hyper_scripter::path::set_home(home).unwrap();
    let guard = LOCK.lock().unwrap_or_else(|err| err.into_inner());
    let _ = env_logger::try_init();
    match std::fs::remove_dir_all(HOME) {
        Ok(_) => (),
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                panic!("重整測試用資料夾失敗了……")
            }
        }
    }
    std::fs::create_dir(HOME).unwrap();
    run("alias e edit --fast").unwrap();

    guard
}
fn join_path(p: &[&str]) -> PathBuf {
    let mut file = get_home();
    for p in p.iter() {
        file = file.join(p);
    }
    file
}

pub fn read(p: &[&str]) -> String {
    let file = join_path(p);
    let s = std::fs::read(file).unwrap();
    let s: &str = std::str::from_utf8(&s).unwrap();
    s.to_owned()
}
pub fn check_exist(p: &[&str]) -> bool {
    let file = join_path(p);
    file.exists()
}
pub fn run<T: ToString>(args: T) -> Result<String, ExitStatus> {
    let home = get_home();
    run_with_home(&*home.to_string_lossy(), args)
}
pub fn run_with_home<T: ToString>(home: &str, args: T) -> Result<String, ExitStatus> {
    let mut full_args = vec!["-H", home];
    let args = args.to_string();
    let args_vec: Vec<&str> = if args.find("|").is_some() {
        let (first, second) = args.split_once("|").unwrap();
        let mut v: Vec<_> = first.split(" ").filter(|s| s.len() > 0).collect();
        v.push(second.trim());
        v
    } else {
        args.split(" ").collect()
    };
    full_args.extend(&args_vec);

    let mut cmd = Command::new(EXE);
    let mut child = cmd
        .args(&full_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let stdout = child.stdout.as_mut().unwrap();
    let mut out_str = vec![];
    let reader = BufReader::new(stdout);
    reader
        .lines()
        .filter_map(|line| line.ok())
        .for_each(|line| {
            println!("{}", line);
            out_str.push(line);
        });

    let status = child.wait().unwrap();
    let res = if status.success() {
        Ok(out_str.join("\n"))
    } else {
        Err(status)
    };
    log::trace!("執行 {:?} 完畢，結果為 {:?}", args_vec, res);
    res
}

pub fn assert_ls_len(expect: usize) {
    let ls_res = run("ls -f all --grouping none --plain --name").unwrap();
    let ls_vec = ls_res
        .split(" ")
        .filter(|s| s.len() > 0)
        .collect::<Vec<_>>();
    assert_eq!(expect, ls_vec.len(), "ls 結果為 {:?}", ls_vec);
}
