use hyper_scripter::{
    config::{Config, PromptLevel},
    path::normalize_path,
};
use std::fs::canonicalize;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Mutex, MutexGuard, Once};

lazy_static::lazy_static! {
    static ref LOCK: Mutex<()> = Mutex::new(());
}
#[cfg(not(debug_assertions))]
const EXE: &'static str = "../target/release/hs";
#[cfg(debug_assertions)]
const EXE: &str = "../target/debug/hs";

pub fn get_exe_abs() -> String {
    canonicalize(EXE)
        .unwrap()
        .to_string_lossy()
        .as_ref()
        .to_owned()
}

const HOME: &str = "./.hyper_scripter";

#[derive(Debug)]
enum ErrorInner {
    Other,
    ExitStatus(ExitStatus),
}
#[derive(Debug)]
pub struct Error {
    msg: Vec<String>,
    inner: ErrorInner,
}
impl Error {
    pub fn exit_status(e: ExitStatus) -> Error {
        Error {
            msg: vec![],
            inner: ErrorInner::ExitStatus(e),
        }
    }
    pub fn other<T: ToString>(s: T) -> Error {
        Error {
            msg: vec![s.to_string()],
            inner: ErrorInner::Other,
        }
    }
    pub fn context<T: ToString>(mut self, s: T) -> Error {
        self.msg.push(s.to_string());
        self
    }
}
type Result<T = ()> = std::result::Result<T, Error>;

pub fn get_home() -> PathBuf {
    normalize_path(HOME).unwrap()
}
pub fn load_conf() -> Config {
    Config::load(hyper_scripter::path::get_home()).unwrap()
}
pub fn setup<'a>() -> MutexGuard<'a, ()> {
    let g = setup_with_utils();
    run("rm --purge * -f all").unwrap();
    g
}
pub fn setup_with_utils<'a>() -> MutexGuard<'a, ()> {
    let guard = LOCK.lock().unwrap_or_else(|err| err.into_inner());
    let _ = env_logger::try_init();
    let home: PathBuf = get_home();
    match std::fs::remove_dir_all(&home) {
        Ok(_) => (),
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                panic!("重整測試用資料夾 {:?} 失敗了……", home);
            }
        }
    }

    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        hyper_scripter::path::set_home(Some(&home)).unwrap();
        Config::init().unwrap();
        Config::set_prompt_level(Some(PromptLevel::Never));
    });

    run_with_home(HOME, "alias e edit --fast").unwrap();

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
pub fn run<T: ToString>(args: T) -> Result<String> {
    let home = get_home();
    run_with_home(&*home.to_string_lossy(), args)
}
pub fn run_with_home<T: ToString>(home: &str, args: T) -> Result<String> {
    let mut full_args = vec!["-H", home, "--prompt-level", "never"];
    let args = args.to_string();
    let args_vec: Vec<&str> = if args.find('|').is_some() {
        let (first, second) = args.split_once("|").unwrap();
        let mut v: Vec<_> = first.split(' ').filter(|s| !s.is_empty()).collect();
        v.push(second.trim());
        v
    } else {
        args.split_whitespace().collect()
    };
    full_args.extend(&args_vec);

    log::info!("開始執行 {:?}", args_vec);
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
        Err(Error::exit_status(status).context(format!("執行 {:?} 失敗", args_vec)))
    };
    log::info!("執行 {:?} 完畢，結果為 {:?}", args_vec, res);
    res
}

fn get_ls(filter: Option<&str>, query: Option<&str>) -> Vec<String> {
    let ls_res = run(format!(
        "ls {} --grouping none --plain --name {}",
        filter.map(|f| format!("-f {}", f)).unwrap_or_default(),
        query.unwrap_or_default()
    ))
    .unwrap();
    ls_res
        .split(' ')
        .filter_map(|s| {
            if !s.is_empty() {
                Some(s.to_owned())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
}
pub fn assert_ls_len(expect: usize, filter: Option<&str>, query: Option<&str>) {
    let res = get_ls(filter, query);
    assert_eq!(expect, res.len(), "ls {:?} 結果為 {:?}", filter, res);
}
pub fn assert_ls(mut expect: Vec<&str>, filter: Option<&str>, query: Option<&str>) {
    expect.sort_unstable();
    let mut res = get_ls(filter, query);
    res.sort();
    assert_eq!(expect, res, "ls {:?} 結果為 {:?}", filter, res);
}

// TODO: 把整合測試的大部份地方改用這個結構
pub struct ScriptTest {
    name: String,
}
impl ScriptTest {
    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn new(name: &str, tags: Option<&str>) -> Self {
        let tags_str = tags.map(|s| format!("-t {}", s)).unwrap_or_default();
        run(format!("e {} ={} | echo $HS_TAGS", tags_str, name)).unwrap();
        ScriptTest {
            name: name.to_owned(),
        }
    }
    pub fn assert_not_exist(&self, args: Option<&str>, msg: Option<&str>) {
        let s = format!("cat {} ={}", args.unwrap_or_default(), self.name);
        let msg = msg.map(|s| format!("\n{}", s)).unwrap_or_default();
        run(&s).expect_err(&format!("{} 找到東西{}", s, msg));
    }
    pub fn archaeology<'a>(&'a self) -> ScriptTestWithFilter<'a> {
        ScriptTestWithFilter {
            script: self,
            filter: "-A",
        }
    }
    pub fn filter<'a>(&'a self, filter: &'a str) -> ScriptTestWithFilter<'a> {
        ScriptTestWithFilter {
            script: self,
            filter,
        }
    }
    pub fn run(&self, args: &str) -> Result<String> {
        self.filter("").run(args)
    }
    pub fn assert_tags<const N: usize>(
        &self,
        tags: [&str; N],
        args: Option<&str>,
        msg: Option<&str>,
    ) {
        let s = format!("{} ={}", args.unwrap_or_default(), self.name);
        let msg = msg.map(|s| format!("\n{}", s)).unwrap_or_default();

        let res = run(&s).expect(&format!("執行 {} 失敗{}", s, msg));
        let mut actual_tags: Vec<_> = res.split(' ').filter(|s| !s.is_empty()).collect();
        actual_tags.sort();
        let mut expected_tags: Vec<_> = tags.iter().map(|s| *s).collect();
        expected_tags.sort();
        assert_eq!(
            expected_tags, actual_tags,
            "{} 的標籤不如預期{}",
            self.name, msg
        );
    }
    pub fn can_find(&self, command: &str) -> Result {
        self.filter("").can_find(command)
    }
    pub fn can_find_by_name(&self) -> Result {
        self.filter("").can_find_by_name()
    }
}
pub struct ScriptTestWithFilter<'a> {
    script: &'a ScriptTest,
    filter: &'a str,
}
impl<'a> ScriptTestWithFilter<'a> {
    pub fn run(&self, args: &str) -> Result<String> {
        let s = format!("{} ={} {}", self.filter, self.script.name, args);
        run(&s)
    }
    pub fn can_find(&self, command: &str) -> Result {
        let command = format!(
            "{} ls --plain --grouping=none --name {}",
            self.filter, command
        );
        let res = run(&command)?;
        if res == self.script.name {
            Ok(())
        } else {
            Err(Error::other(format!(
                "想找 {} 卻找到 {}",
                self.script.name, res
            )))
        }
    }
    pub fn can_find_by_name(&self) -> Result {
        self.can_find(&format!("={}", self.script.name))
    }
}
