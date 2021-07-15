use crate::config::Config;
use crate::error::{Contextable, Error, Result};
use crate::path::{get_home, get_template_path};
use crate::script::ScriptInfo;
use crate::script_type::{get_default_template, ScriptType};
use chrono::{DateTime, Utc};
use std::ffi::OsStr;
use std::fs::{create_dir_all, remove_file, rename, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

pub mod main_util;

// XXX: main util?
pub fn run(
    script_path: &Path,
    info: &ScriptInfo,
    remaining: &[String],
    content: &str,
) -> Result<()> {
    let conf = Config::get();
    let ty = &info.ty;
    let name = &info.name.key();
    let hs_home = get_home();
    let hs_tags: Vec<_> = info.tags.iter().map(|t| t.as_ref()).collect();

    let hs_exe = std::env::current_exe()?;
    let hs_exe = hs_exe.to_string_lossy();

    let hs_cmd = std::env::args().next().unwrap_or_default();

    let script_conf = conf.get_script_conf(ty)?;
    let cmd_str = if let Some(cmd) = &script_conf.cmd {
        cmd
    } else {
        return Err(Error::PermissionDenied(vec![script_path.to_path_buf()]));
    };

    macro_rules! remaining_iter {
        () => {
            remaining.iter().map(|s| AsRef::<OsStr>::as_ref(s))
        };
    }

    let info: serde_json::Value;
    info = json!({
        "path": script_path,
        "hs_home": hs_home,
        "hs_tags": hs_tags,
        "hs_cmd": hs_cmd,
        "hs_exe": hs_exe,
        "name": name,
        "content": content,
    });
    let env = conf.gen_env(&info)?;
    let ty_env = script_conf.gen_env(&info)?;

    if let Some(pre_run_script) = find_pre_run() {
        let args = std::iter::once(pre_run_script.as_ref()).chain(remaining_iter!());
        let mut cmd = create_cmd("sh", args);
        cmd.envs(ty_env.iter().map(|(a, b)| (a, b)));
        cmd.envs(env.iter().map(|(a, b)| (a, b)));

        let stat = run_cmd(cmd)?;
        log::info!("預腳本執行結果：{:?}", stat);
        if !stat.success() {
            // TODO: 根據返回值做不同表現
            let code = stat.code().unwrap_or_default();
            return Err(Error::PreRunError(code));
        }
    }

    let args = script_conf.args(&info)?;
    let full_args: Vec<&OsStr> = args
        .iter()
        .map(|s| s.as_ref())
        .chain(remaining_iter!())
        .collect();

    let mut cmd = create_cmd(&cmd_str, &full_args);
    cmd.envs(ty_env);
    cmd.envs(env);

    let stat = run_cmd(cmd)?;
    log::info!("程式執行結果：{:?}", stat);
    if !stat.success() {
        let code = stat.code().unwrap_or_default();
        Err(Error::ScriptError(code))
    } else {
        Ok(())
    }
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
pub fn change_name_only<F: Fn(&str) -> String>(full_name: &str, transform: F) -> String {
    let mut arr: Vec<_> = full_name.split('/').collect();
    let len = arr.len();
    if len == 0 {
        unreachable!();
    } else if len == 1 {
        transform(full_name)
    } else {
        let new = transform(arr[len - 1]);
        arr[len - 1] = &new;
        arr.join("/")
    }
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
    let p = path.iter().map(|p| p.as_ref().to_owned()).collect();
    log::debug!("檔案系統錯誤：{:?}, {:?}", p, err);
    match err.kind() {
        std::io::ErrorKind::PermissionDenied => Error::PermissionDenied(p),
        std::io::ErrorKind::NotFound => Error::PathNotFound(p),
        _ => Error::GeneralFS(p, Arc::new(err)),
    }
}
pub fn handle_fs_res<T, P: AsRef<Path>>(path: &[P], res: std::io::Result<T>) -> Result<T> {
    match res {
        Ok(t) => Ok(t),
        Err(e) => Err(handle_fs_err(path, e)),
    }
}

fn get_or_create_tamplate(ty: &ScriptType) -> Result<String> {
    let tmpl_path = get_template_path(ty)?;
    if tmpl_path.exists() {
        return read_file(&tmpl_path);
    }
    let default_tmpl = get_default_template(ty);
    write_file(&tmpl_path, default_tmpl)?;
    Ok(default_tmpl.to_owned())
}

// 如果有需要跳脫的字元就吐 json 格式，否則就照原字串
pub fn to_display_args(arg: String) -> Result<String> {
    let mut need_escape = false;
    for ch in arg.chars() {
        match ch {
            ' ' | '>' | '|' | '\'' | '#' | '<' | ';' | '(' | ')' | '{' | '}' | '$' => {
                need_escape = true
            }
            _ => (),
        }
    }

    let escaped: String =
        serde_json::to_string(&arg).context("超級異常的狀況…把字串轉成 json 也能出錯")?;
    if need_escape || arg != escaped[1..escaped.len() - 1] {
        Ok(escaped)
    } else {
        Ok(arg)
    }
}

pub fn serialize_to_string<S: serde::Serializer, T: ToString>(
    t: T,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    serializer.serialize_str(&t.to_string())
}

fn relative_to_home(p: &Path) -> Option<&Path> {
    const CUR_DIR: &str = ".";
    let home = dirs::home_dir()?;
    if p == home {
        return Some(CUR_DIR.as_ref());
    }
    p.strip_prefix(&home).ok()
}

fn find_pre_run() -> Option<PathBuf> {
    use crate::path;
    let p = path::get_home().join(path::HS_PRE_RUN);
    if p.exists() {
        log::info!("找到預執行腳本 {:?}", p);
        Some(p)
    } else {
        None
    }
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
        let birthplace_abs = handle_fs_res(&["."], std::env::current_dir())?;
        let birthplace = relative_to_home(&birthplace_abs);

        // NOTE: 創建資料夾和檔案
        if let Some(parent) = path.parent() {
            handle_fs_res(&[path], create_dir_all(parent))?;
        }
        let mut file = handle_fs_res(&[path], File::create(&path))?;

        let content = content.iter().map(|s| s.as_ref().split('\n')).flatten();
        if !no_template {
            let content: Vec<_> = content.collect();
            let info = json!({
                "birthplace_in_home": birthplace.is_some(),
                "birthplace": birthplace,
                "birthplace_abs": birthplace_abs,
                "name": script.name.key().to_owned(),
                "content": content,
            });
            let template = get_or_create_tamplate(&script.ty)?;
            handle_fs_res(&[path], write_prepare_script(file, &template, &info))?;
        } else {
            for line in content {
                writeln!(file, "{}", line)?;
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
