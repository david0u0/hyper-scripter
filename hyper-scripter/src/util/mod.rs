use crate::color::{Color, Stylize};
use crate::config::Config;
use crate::error::{Contextable, Error, FormatCode::Template as TemplateCode, Result};
use crate::extract_msg::Message;
use crate::path;
use crate::script::ScriptInfo;
use crate::script_type::{get_default_template, ScriptFullType, ScriptType};
use ::serde::Serialize;
use chrono::{DateTime, Utc};
use shlex::Shlex;
use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs::{create_dir_all, remove_file, rename, File};
use std::io::{self, BufRead};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub mod completion_util;
pub mod holder;
pub mod main_util;
pub mod shebang_handle;
pub mod writable;

pub mod init_repo;
pub use init_repo::*;

pub mod serde;
pub(crate) use self::serde::impl_de_by_from_str;
pub(crate) use self::serde::impl_ser_and_display_by_as_ref;
pub(crate) use self::serde::impl_ser_by_to_string;

pub fn illegal_name(s: &str) -> bool {
    s.starts_with('-')
        || s.starts_with('.')
        || s.contains("..")
        || s.contains(' ')
        || s.contains('@')
        || s.contains('*')
        || s.contains('!')
        || s.contains('?')
        || s.contains('=')
        || s.contains('/')
        || s.is_empty()
}

pub fn run_cmd(mut cmd: Command) -> Result<Option<i32>> {
    log::debug!("執行命令 {:?}", cmd);
    let res = cmd.spawn();
    let program = cmd.get_program();
    let mut child = handle_fs_res(&[program], res)?;
    let stat = handle_fs_res(&[program], child.wait())?;
    if stat.success() {
        Ok(None)
    } else {
        Ok(Some(stat.code().unwrap_or_default()))
    }
}
#[cfg(not(target_os = "linux"))]
pub fn create_cmd(cmd_str: &str, args: &[impl AsRef<OsStr>]) -> Command {
    log::debug!("在非 linux 上執行，用 sh -c 包一層");
    let args: Vec<_> = args
        .iter()
        .map(|s| {
            s.as_ref()
                .to_str()
                .unwrap()
                .to_string()
                .replace(r"\", r"\\\\")
        })
        .collect();
    let arg = format!("{} {}", cmd_str, args.join(" "));
    let mut cmd = Command::new("sh");
    cmd.args(&["-c", &arg]);
    cmd
}
#[cfg(target_os = "linux")]
pub fn create_cmd<I, S1, S2>(cmd_str: S2, args: I) -> Command
where
    I: IntoIterator<Item = S1>,
    S1: AsRef<OsStr>,
    S2: AsRef<OsStr>,
{
    let mut cmd = Command::new(&cmd_str);
    cmd.args(args);
    cmd
}

pub fn run_shell(args: &[String]) -> Result<i32> {
    let cmd = args.join(" ");
    log::debug!("shell args = {:?}", cmd);
    let mut cmd = create_cmd("sh", ["-c", &cmd]);
    let env = Config::get().gen_env(&TmplVal::new(), false)?;
    cmd.envs(env.iter().map(|(a, b)| (a, b)));
    let code = run_cmd(cmd)?;
    Ok(code.unwrap_or_default())
}

pub fn open_editor<'a>(path: impl IntoIterator<Item = &'a Path>) -> Result {
    let conf = Config::get();
    let editor = conf.editor.iter().map(|s| Cow::Borrowed(s.as_ref()));
    let cmd = create_concat_cmd(editor, path);
    let code = run_cmd(cmd)?;
    if let Some(code) = code {
        return Err(Error::EditorError(code, conf.editor.clone()));
    }
    Ok(())
}

pub fn create_concat_cmd_shlex<'b, I2, S2>(arg1: &str, arg2: I2) -> Command
where
    I2: IntoIterator<Item = &'b S2>,
    S2: AsRef<OsStr> + 'b + ?Sized,
{
    let arg1 = Shlex::new(arg1).map(|s| Cow::Owned(s.into()));
    create_concat_cmd(arg1, arg2)
}

pub fn create_concat_cmd<'a, I1, I2, S2>(arg1: I1, arg2: I2) -> Command
where
    I1: IntoIterator<Item = Cow<'a, OsStr>>,
    I2: IntoIterator<Item = &'a S2>,
    S2: AsRef<OsStr> + 'a + ?Sized,
{
    let mut arg1 = arg1.into_iter();
    let cmd = arg1.next().unwrap();
    let remaining = arg1.chain(arg2.into_iter().map(|s| Cow::Borrowed(s.as_ref())));
    create_cmd(cmd, remaining)
}

pub fn file_modify_time(path: &Path) -> Result<DateTime<Utc>> {
    let meta = handle_fs_res(&[path], std::fs::metadata(path))?;
    let modified = handle_fs_res(&[path], meta.modified())?;
    Ok(modified.into())
}

pub fn read_file_lines(path: &Path) -> Result<impl Iterator<Item = String>> {
    let file = handle_fs_res(&[path], File::open(path)).context("唯讀開啟檔案失敗")?;
    Ok(io::BufReader::new(file)
        .lines()
        .map(|s| s.unwrap_or_default()))
}

pub fn read_file(path: &Path) -> Result<String> {
    let mut file = handle_fs_res(&[path], File::open(path)).context("唯讀開啟檔案失敗")?;
    let mut content = String::new();
    handle_fs_res(&[path], file.read_to_string(&mut content)).context("讀取檔案失敗")?;
    Ok(content)
}

pub fn write_file(path: &Path, content: &str) -> Result<()> {
    let mut file = handle_fs_res(&[path], File::create(path))?;
    handle_fs_res(&[path], file.write_all(content.as_bytes()))
}
pub fn remove(script_path: &Path) -> Result<()> {
    handle_fs_res(&[&script_path], remove_file(&script_path))
}
pub fn mv(origin: &Path, new: &Path) -> Result<()> {
    log::info!("修改 {:?} 為 {:?}", origin, new);
    // NOTE: 創建資料夾和檔案
    if let Some(parent) = new.parent() {
        handle_fs_res(&[&new], create_dir_all(parent))?;
    }
    handle_fs_res(&[&new, &origin], rename(&origin, &new))
}
pub fn cp(origin: &Path, new: &Path) -> Result<()> {
    // NOTE: 創建資料夾和檔案
    if let Some(parent) = new.parent() {
        handle_fs_res(&[parent], create_dir_all(parent))?;
    }
    let _copied = handle_fs_res(&[&origin, &new], std::fs::copy(&origin, &new))?;
    Ok(())
}

pub fn handle_fs_err<P: AsRef<Path>>(path: &[P], err: std::io::Error) -> Error {
    use std::sync::Arc;
    let mut p = path.iter().map(|p| p.as_ref().to_owned()).collect();
    log::warn!("檔案系統錯誤：{:?}, {:?}", p, err);
    match err.kind() {
        std::io::ErrorKind::PermissionDenied => Error::PermissionDenied(p),
        std::io::ErrorKind::NotFound => Error::PathNotFound(p),
        std::io::ErrorKind::AlreadyExists => Error::PathExist(p.remove(0)),
        _ => Error::GeneralFS(p, Arc::new(err)),
    }
}
pub fn handle_fs_res<T, P: AsRef<Path>>(path: &[P], res: std::io::Result<T>) -> Result<T> {
    match res {
        Ok(t) => Ok(t),
        Err(e) => Err(handle_fs_err(path, e)),
    }
}

/// check_subtype 是為避免太容易生出子模版
pub fn get_or_create_template_path(
    ty: &ScriptFullType,
    force: bool,
    check_subtype: bool,
) -> Result<(PathBuf, Option<&'static str>)> {
    if !force {
        Config::get().get_script_conf(&ty.ty)?; // 確認類型存在與否
    }
    let tmpl_path = path::get_template_path(ty)?;
    if !tmpl_path.exists() {
        if check_subtype && ty.sub.is_some() {
            return Err(Error::UnknownType(ty.to_string()));
        }
        let default_tmpl = get_default_template(ty);
        return write_file(&tmpl_path, default_tmpl).map(|_| (tmpl_path, Some(default_tmpl)));
    }
    Ok((tmpl_path, None))
}
pub fn get_or_create_template(
    ty: &ScriptFullType,
    force: bool,
    check_subtype: bool,
) -> Result<String> {
    let (tmpl_path, default_tmpl) = get_or_create_template_path(ty, force, check_subtype)?;
    if let Some(default_tmpl) = default_tmpl {
        return Ok(default_tmpl.to_owned());
    }
    read_file(&tmpl_path)
}

fn relative_to_home(p: &Path) -> Option<&Path> {
    const CUR_DIR: &str = ".";
    let home = dirs::home_dir()?;
    if p == home {
        return Some(CUR_DIR.as_ref());
    }
    p.strip_prefix(&home).ok()
}

fn get_birthplace() -> Result<PathBuf> {
    // NOTE: 用 $PWD 可以取到 symlink 還沒解開前的路徑
    // 若用 std::env::current_dir，該路徑已為真實路徑
    let here = std::env::var("PWD")?;
    Ok(here.into())
}

pub fn compute_hash(msg: &str) -> i64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new(); // TODO: other hash function maybe?
    msg.hash(&mut hasher);
    let hash = hasher.finish();
    i64::from_ne_bytes(hash.to_ne_bytes())
}
pub fn compute_file_hash(path: &Path) -> Result<i64> {
    read_file(path).map(|c| compute_hash(&c))
}

#[derive(Debug, Clone, Copy)]
pub enum PrepareRespond {
    New { create_time: DateTime<Utc> },
    Old { last_hash: i64 },
}
pub fn prepare_script<T: AsRef<str>>(
    path: &Path,
    script: &ScriptInfo,
    template: Option<String>,
    content: &[T],
) -> Result<PrepareRespond> {
    log::info!("開始準備 {} 腳本內容……", script.name);
    let has_content = !content.is_empty();
    let is_new = !path.exists();
    if is_new {
        let birthplace = get_birthplace()?;
        let birthplace_rel = relative_to_home(&birthplace);

        let mut file = handle_fs_res(&[path], File::create(&path))?;

        let content = content.iter().map(|s| s.as_ref().split('\n')).flatten();
        if let Some(template) = template {
            let content: Vec<_> = content.collect();
            let info = json!({
                "birthplace_in_home": birthplace_rel.is_some(),
                "birthplace_rel": birthplace_rel,
                "birthplace": birthplace,
                "name": script.name.key().to_owned(),
                "content": content,
            });
            log::debug!("編輯模版資訊：{:?}", info);
            write_prepare_script(file, &path, &template, &info)?;
        } else {
            let mut first = true;
            for line in content {
                if !first {
                    writeln!(file, "")?;
                }
                first = false;
                write!(file, "{}", line)?;
            }
        }

        Ok(PrepareRespond::New {
            create_time: file_modify_time(path)?,
        })
    } else {
        if has_content {
            log::debug!("腳本已存在，往後接上給定的訊息");
            let mut file = handle_fs_res(
                &[path],
                std::fs::OpenOptions::new()
                    .append(true)
                    .write(true)
                    .open(path),
            )?;
            for content in content.iter() {
                handle_fs_res(&[path], writeln!(&mut file, "{}", content.as_ref()))?;
            }
        }
        Ok(PrepareRespond::Old {
            last_hash: script.hash,
        })
    }
}
fn write_prepare_script<W: Write>(
    w: W,
    path: &Path,
    template: &str,
    info: &serde_json::Value,
) -> Result {
    use handlebars::{Handlebars, TemplateRenderError};
    let reg = Handlebars::new();
    reg.render_template_to_write(&template, &info, w)
        .map_err(|err| match err {
            TemplateRenderError::TemplateError(err) => {
                log::warn!("解析模版錯誤：{}", err);
                TemplateCode.to_err(template.to_owned())
            }
            TemplateRenderError::IOError(err, ..) => handle_fs_err(&[path], err),
            TemplateRenderError::RenderError(err) => err.into(),
        })
}

/// 可用來表示「未知類別」的概念 TODO: 測試之
pub struct DisplayType<'a> {
    ty: &'a ScriptType,
    color: Option<Color>,
}
impl<'a> DisplayType<'a> {
    pub fn is_unknown(&self) -> bool {
        self.color.is_none()
    }
    pub fn color(&self) -> Color {
        self.color.unwrap_or(Color::BrightBlack)
    }
    pub fn display(&self) -> Cow<'a, str> {
        if self.is_unknown() {
            Cow::Owned(format!("{}, unknown", self.ty))
        } else {
            Cow::Borrowed(self.ty.as_ref())
        }
    }
}
pub fn get_display_type(ty: &ScriptType) -> DisplayType<'_> {
    let conf = Config::get();
    match conf.get_color(ty) {
        Err(e) => {
            log::warn!("取腳本顏色時出錯：{}，視為未知類別", e);
            DisplayType { ty, color: None }
        }
        Ok(c) => DisplayType { ty, color: Some(c) },
    }
}

pub fn print_iter<T: std::fmt::Display>(iter: impl Iterator<Item = T>, sep: &str) -> bool {
    let mut first = true;
    for t in iter {
        if !first {
            print!("{}", sep);
        }
        first = false;
        print!("{}", t);
    }
    !first
}

pub fn option_map_res<T, F: FnOnce(T) -> Result<T>>(opt: Option<T>, f: F) -> Result<Option<T>> {
    Ok(match opt {
        Some(t) => Some(f(t)?),
        None => None,
    })
}

pub fn hijack_ctrlc_once() {
    use std::sync::Once;
    static CTRLC_HANDLE: Once = Once::new();
    log::debug!("劫持 ctrl-c 回調");
    CTRLC_HANDLE.call_once(|| {
        let res = ctrlc::set_handler(|| log::warn!("收到 ctrl-c"));
        if res.is_err() {
            log::warn!("設置 ctrl-c 回調失敗 {:?}", res);
        }
    });
}

pub fn prompt(msg: impl std::fmt::Display, allow_enter: bool) -> Result<bool> {
    use console::{Key, Term};

    enum Res {
        Y,
        N,
        Exit,
    }

    fn inner(term: &Term, msg: &str, allow_enter: bool) -> Result<Res> {
        term.hide_cursor()?;
        hijack_ctrlc_once();

        let res = loop {
            term.write_str(msg)?;
            match term.read_key() {
                Ok(Key::Char('Y' | 'y')) => break Res::Y,
                Ok(Key::Enter) => {
                    if allow_enter {
                        break Res::Y;
                    } else {
                        term.write_line("")?;
                    }
                }
                Ok(Key::Char('N' | 'n')) => break Res::N,
                Ok(Key::Char(ch)) => term.write_line(&format!(" Unknown key '{}'", ch))?,
                Ok(Key::Escape) => {
                    break Res::Exit;
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::Interrupted {
                        break Res::Exit;
                    } else {
                        return Err(e.into());
                    }
                }
                _ => term.write_line(" Unknown key")?,
            }
        };
        Ok(res)
    }

    let term = Term::stderr();
    let msg = if allow_enter {
        format!("{} [Y/Enter/N]", msg)
    } else {
        format!("{} [Y/N]", msg)
    };
    let res = inner(&term, &msg, allow_enter);
    term.show_cursor()?;
    match res? {
        Res::Exit => {
            std::process::exit(1);
        }
        Res::Y => {
            term.write_line(&" Y".stylize().color(Color::Green).to_string())?;
            Ok(true)
        }
        Res::N => {
            term.write_line(&" N".stylize().color(Color::Red).to_string())?;
            Ok(false)
        }
    }
}

#[derive(Serialize)]
pub struct TmplVal<'a> {
    home: &'static Path,
    cmd: String,
    exe: PathBuf,
    editor: &'static [String],

    path: Option<&'a Path>,
    run_id: Option<i64>,
    tags: Vec<&'a str>,
    env_desc: Vec<Message>,
    name: Option<&'a str>,
}
impl<'a> TmplVal<'a> {
    pub fn new() -> Self {
        TmplVal {
            home: path::get_home(),
            cmd: std::env::args().next().unwrap_or_default(),
            exe: std::env::current_exe().unwrap_or_default(),
            editor: &Config::get().editor,

            path: None,
            run_id: None,
            tags: vec![],
            env_desc: vec![],
            name: None,
        }
    }
}
