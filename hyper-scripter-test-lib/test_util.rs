use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::ExitStatus;
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard};

lazy_static::lazy_static! {
    static ref LOCK: Mutex<()> = Mutex::new(());
}
#[cfg(not(debug_assertions))]
const EXE: &'static str = "../target/release/hyper-scripter";
#[cfg(debug_assertions)]
const EXE: &'static str = "../target/debug/hyper-scripter";

pub fn get_exe_abs() -> String {
    std::fs::canonicalize(EXE)
        .unwrap()
        .to_string_lossy()
        .as_ref()
        .to_owned()
}

const PATH: &str = "./.hyper_scripter";

pub fn get_path() -> PathBuf {
    PATH.into()
}

pub fn setup<'a>() -> MutexGuard<'a, ()> {
    let guard = LOCK.lock().unwrap_or_else(|err| err.into_inner());
    let _ = env_logger::try_init();
    match std::fs::remove_dir_all(PATH) {
        Ok(_) => (),
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                panic!("重整測試用資料夾失敗了……")
            }
        }
    }
    let _ = std::fs::remove_dir_all(".tmp");

    guard
}
fn join_path(p: &[&str]) -> PathBuf {
    let mut file: PathBuf = PATH.into();
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
pub fn run(args: &[&str]) -> Result<String, ExitStatus> {
    run_with_home(PATH, args)
}
pub fn run_with_home(home: &str, args: &[&str]) -> Result<String, ExitStatus> {
    let mut full_args = vec!["-p", home];
    full_args.extend(args);

    let mut cmd = Command::new(EXE);
    let mut child = cmd.args(&full_args).stdout(Stdio::piped()).spawn().unwrap();
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
    log::trace!("執行 {:?} 完畢，結果為 {:?}", args, res);
    res
}
