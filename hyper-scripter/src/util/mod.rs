use crate::config::Config;
use crate::error::{Contextable, Error, Result};
use crate::path;
use crate::script::ScriptInfo;
use crate::script_type::{get_default_template, ScriptType};
use chrono::{DateTime, Utc};
use colored::Color;
use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs::{create_dir_all, remove_file, rename, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

pub mod completion_util;
pub mod holder;
pub mod main_util;
pub mod serde;

pub mod init_repo;
pub use init_repo::*;

pub fn illegal_name(s: &str) -> bool {
    s.starts_with('-')
        || s.starts_with('.')
        || s.contains("..")
        || s.contains(' ')
        || s.contains('!')
        || s.contains('?')
        || s.contains('=')
        || s.contains('/')
        || s.is_empty()
}

pub fn run_cmd(mut cmd: Command) -> Result<ExitStatus> {
    let res = cmd.spawn();
    let program = cmd.get_program();
    let mut child = handle_fs_res(&[program], res)?;
    handle_fs_res(&[program], child.wait())
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

pub fn open_editor(path: &Path) -> Result {
    let conf = Config::get();
    let cmd = create_concat_cmd(&conf.editor, &[&path]);
    let stat = run_cmd(cmd)?;
    if !stat.success() {
        let code = stat.code().unwrap_or_default();
        return Err(Error::EditorError(code, conf.editor.clone()));
    }
    Ok(())
}

pub fn create_concat_cmd<'a, 'b, I1, S1, I2, S2>(arg1: I1, arg2: I2) -> Command
where
    I1: IntoIterator<Item = &'a S1>,
    I2: IntoIterator<Item = &'b S2>,
    S1: AsRef<OsStr> + 'a,
    S2: AsRef<OsStr> + 'b,
{
    let mut arg1 = arg1.into_iter();
    let cmd = arg1.next().unwrap();
    let remaining = arg1
        .map(|s| s.as_ref())
        .chain(arg2.into_iter().map(|s| s.as_ref()));
    create_cmd(cmd, remaining)
}

pub fn file_modify_time(path: &Path) -> Result<DateTime<Utc>> {
    let meta = handle_fs_res(&[path], std::fs::metadata(path))?;
    let modified = handle_fs_res(&[path], meta.modified())?;
    Ok(modified.into())
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

pub fn get_template_path(ty: &ScriptType, force: bool) -> Result<PathBuf> {
    if !force {
        Config::get().get_script_conf(&ty)?; // 確認類型存在與否
    }
    path::get_template_path(ty)
}
pub fn get_or_create_tamplate(ty: &ScriptType, force: bool) -> Result<String> {
    let tmpl_path = get_template_path(ty, force)?;
    if tmpl_path.exists() {
        return read_file(&tmpl_path);
    }
    let default_tmpl = get_default_template(ty);
    write_file(&tmpl_path, default_tmpl)?;
    Ok(default_tmpl.to_owned())
}

// 如果有需要跳脫的字元就吐 json 格式，否則就照原字串
pub fn to_display_args(arg: String) -> String {
    let mut need_escape = false;
    for ch in arg.chars() {
        match ch {
            ' ' | '>' | '|' | '\'' | '#' | '<' | ';' | '(' | ')' | '{' | '}' | '$' => {
                need_escape = true
            }
            _ => (),
        }
    }

    let escaped: String = serde_json::to_string(&arg).unwrap();
    if need_escape || arg != escaped[1..escaped.len() - 1] {
        escaped
    } else {
        arg
    }
}

fn relative_to_home(p: &Path) -> Option<&Path> {
    const CUR_DIR: &str = ".";
    let home = dirs::home_dir()?;
    if p == home {
        return Some(CUR_DIR.as_ref());
    }
    p.strip_prefix(&home).ok()
}

#[derive(Debug)]
pub enum PrepareRespond {
    HasContent,
    NoContent { is_new: bool, time: DateTime<Utc> },
}
pub fn prepare_script<T: AsRef<str>>(
    path: &Path,
    script: &ScriptInfo,
    no_template: bool,
    content: &[T],
) -> Result<PrepareRespond> {
    log::info!("開始準備 {} 腳本內容……", script.name);
    let has_content = !content.is_empty();
    let is_new = !path.exists();
    if is_new {
        let birthplace = path::normalize_path(".")?;
        let birthplace_rel = relative_to_home(&birthplace);

        // NOTE: 創建資料夾和檔案
        if let Some(parent) = path.parent() {
            handle_fs_res(&[path], create_dir_all(parent))?;
        }
        let mut file = handle_fs_res(&[path], File::create(&path))?;

        let content = content.iter().map(|s| s.as_ref().split('\n')).flatten();
        if !no_template {
            let content: Vec<_> = content.collect();
            let info = json!({
                "birthplace_in_home": birthplace_rel.is_some(),
                "birthplace_rel": birthplace_rel,
                "birthplace": birthplace,
                "name": script.name.key().to_owned(),
                "content": content,
            });
            // NOTE: 計算 `path` 時早已檢查過腳本類型，這裡直接不檢查了
            let template = get_or_create_tamplate(&script.ty, true)?;
            handle_fs_res(&[path], write_prepare_script(file, &template, &info))?;
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
    }

    Ok(if has_content {
        PrepareRespond::HasContent
    } else {
        PrepareRespond::NoContent {
            is_new,
            time: file_modify_time(path)?,
        }
    })
}
fn write_prepare_script<W: Write>(
    w: W,
    template: &str,
    info: &serde_json::Value,
) -> std::io::Result<()> {
    use handlebars::{Handlebars, TemplateRenderError};
    let reg = Handlebars::new();
    reg.render_template_to_write(&template, &info, w)
        .map_err(|err| match err {
            TemplateRenderError::IOError(err, ..) => err,
            e => panic!("解析模版錯誤：{}", e),
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
pub fn get_display_type(ty: &ScriptType) -> DisplayType {
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
        let res = ctrlc::set_handler(|| {});
        if res.is_err() {
            log::warn!("設置 ctrl-c 回調失敗 {:?}", res);
        }
    });
}
