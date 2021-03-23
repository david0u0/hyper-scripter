use crate::config::Config;
use crate::error::{Contextable, Error, Result, SysPath};
use crate::path::get_home;
use crate::script::ScriptInfo;
use chrono::{DateTime, Utc};
use handlebars::{Handlebars, TemplateRenderError};
use std::ffi::OsStr;
use std::fs::{create_dir_all, remove_file, rename, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

pub fn run(
    script_path: &PathBuf,
    info: &ScriptInfo,
    remaining: &[String],
    content: &str,
) -> Result<()> {
    let ty = &info.ty;
    let script_conf = Config::get()?.get_script_conf(ty)?;
    let cmd_str = if let Some(cmd) = &script_conf.cmd {
        cmd
    } else {
        return Err(Error::PermissionDenied(vec![script_path.clone()]));
    };

    let info: serde_json::Value;
    info = json!({
        "path": script_path,
        "content": content,
    });

    let args = script_conf.args(&info)?;
    let mut full_args: Vec<&OsStr> = args.iter().map(|s| s.as_ref()).collect();
    full_args.extend(remaining.iter().map(|s| AsRef::<OsStr>::as_ref(s)));

    let mut cmd = create_cmd(&cmd_str, &full_args);
    let env = script_conf.env(&info)?;
    cmd.envs(env);

    let stat = run_cmd(&cmd_str, cmd)?;
    log::info!("程式執行結果：{:?}", stat);
    if !stat.success() {
        let code = stat.code().unwrap_or_default();
        Err(Error::ScriptError(code))
    } else {
        Ok(())
    }
}
pub fn run_cmd(cmd_str: &str, mut cmd: Command) -> Result<ExitStatus> {
    let mut child = handle_fs_res(&[&cmd_str], cmd.spawn())?;
    handle_fs_res(&[&cmd_str], child.wait())
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
pub fn create_cmd(cmd_str: &str, args: &[impl AsRef<OsStr>]) -> Command {
    let mut cmd = Command::new(&cmd_str);
    cmd.args(args);
    cmd
}

pub fn read_file(path: &PathBuf) -> Result<String> {
    let mut file = handle_fs_res(&[path], File::open(path)).context("唯讀開啟檔案失敗")?;
    let mut content = String::new();
    handle_fs_res(&[path], file.read_to_string(&mut content)).context("讀取檔案失敗")?;
    Ok(content)
}

pub fn write_file(path: &PathBuf, content: &str) -> Result<()> {
    let mut file = handle_fs_res(&[path], File::create(path))?;
    handle_fs_res(&[path], file.write_all(content.as_bytes()))
}
pub fn remove(script_path: &PathBuf) -> Result<()> {
    handle_fs_res(&[&script_path], remove_file(&script_path))
}
pub fn change_name_only<F: Fn(&str) -> String>(full_name: &str, transform: F) -> String {
    let mut arr: Vec<_> = full_name.split("/").collect();
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
pub fn mv(origin: &PathBuf, new: &PathBuf) -> Result<()> {
    log::info!("修改 {:?} 為 {:?}", origin, new);
    // NOTE: 創建資料夾和檔案
    if let Some(parent) = new.parent() {
        handle_fs_res(&[&new], create_dir_all(parent))?;
    }
    handle_fs_res(&[&new, &origin], rename(&origin, &new))
}
pub fn cp(origin: &PathBuf, new: &PathBuf) -> Result<()> {
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

pub fn prepare_script(
    path: &Path,
    script: &ScriptInfo,
    no_template: bool,
    content: Option<&str>,
) -> Result<Option<DateTime<Utc>>> {
    log::info!("開始準備 {} 腳本內容……", script.name);
    let mut is_new = if path.exists() {
        log::debug!("腳本已存在，不填入預設訊息");
        false
    } else {
        let home = dirs::home_dir().ok_or(Error::SysPathNotFound(SysPath::Home))?;
        let birthplace = handle_fs_res(&["."], std::env::current_dir())?;
        let birthplace = birthplace.strip_prefix(home).unwrap_or(&birthplace);

        // NOTE: 創建資料夾和檔案
        if let Some(parent) = path.parent() {
            handle_fs_res(&[path], create_dir_all(parent))?;
        }
        let mut file = handle_fs_res(&[path], File::create(&path))?;

        let content = content.unwrap_or_default();
        if !no_template {
            let content: Vec<_> = content.split(";").collect();
            let info = json!({
                "script_dir": get_home(),
                "birthplace": birthplace,
                "name": script.name.key().to_owned(),
                "content": content,
            });
            let template = &Config::get()?.get_script_conf(&script.ty)?.template;
            handle_fs_res(&[path], write_prepare_script(file, template, &info))?;
        } else {
            write!(file, "{}", content)?;
        }
        true
    };
    if content.is_some() {
        is_new = false;
    }
    Ok(if is_new { Some(Utc::now()) } else { None })
}
fn write_prepare_script<W: Write>(
    w: W,
    template: &Vec<String>,
    info: &serde_json::Value,
) -> std::io::Result<()> {
    let reg = Handlebars::new();
    let template = template.join("\n");
    reg.render_template_to_write(&template, &info, w)
        .map_err(|err| match err {
            TemplateRenderError::IOError(err, ..) => err,
            e => panic!("解析模版錯誤：{}", e),
        })
}
pub fn after_script(path: &Path, created: Option<DateTime<Utc>>) -> Result<bool> {
    if let Some(created) = created {
        let meta = handle_fs_res(&[path], std::fs::metadata(path))?;
        let modified = handle_fs_res(&[path], meta.modified())?;
        let modified = modified.duration_since(std::time::UNIX_EPOCH)?.as_secs();
        if created.timestamp() >= modified as i64 {
            log::info!("腳本未變動，刪除之");
            handle_fs_res(&[path], remove_file(path))?;
            Ok(false)
        } else {
            log::debug!("腳本更新時間比創建還新，不執行後處理");
            Ok(true)
        }
    } else {
        log::debug!("既存腳本，不執行後處理");
        Ok(true)
    }
}

pub fn to_display_args(arg: String) -> Result<String> {
    let escaped: String = serde_json::from_str(&arg)?;
    if arg == &escaped[1..escaped.len() - 1] {
        Ok(arg)
    } else {
        Ok(escaped)
    }
}

pub mod main_util;
