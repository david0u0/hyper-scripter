use crate::error::{Contextabl, Error, Result};
use crate::script::{ScriptInfo, ScriptMeta, ScriptType};
use crate::templates;
use chrono::{DateTime, Utc};
use handlebars::{Handlebars, TemplateRenderError};
use std::ffi::OsStr;
use std::fs::{remove_file, rename, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

pub fn run(script: &ScriptMeta, info: &ScriptInfo, remaining: &[String]) -> Result<()> {
    let ty = info.ty;
    let (cmd_str, args) = ty
        .cmd()
        .ok_or(Error::Operation(format!("{} is not runnable", ty)))?;
    let mut full_args: Vec<&OsStr> = args.iter().map(|s| s.as_ref()).collect();

    full_args.push(script.path.as_ref());
    full_args.extend(remaining.iter().map(|s| AsRef::<OsStr>::as_ref(s)));
    // TODO: 看要不要把執行狀態傳回去？
    let cmd = create_cmd(&cmd_str, &full_args);
    let stat = run_cmd(&cmd_str, cmd)?;
    log::info!("程式執行結果：{:?}", stat);
    if !stat.success() {
        Err(Error::ScriptError(stat.to_string()))
    } else {
        Ok(())
    }
}
#[cfg(not(target_os = "linux"))]
pub fn run_cmd(cmd_str: &str, mut cmd: Command) -> Result<ExitStatus> {
    let output = handle_fs_res(&[&cmd_str], cmd.output())?;
    println!("{}", std::str::from_utf8(&output.stdout)?);
    Ok(output.status)
}
#[cfg(target_os = "linux")]
pub fn run_cmd(cmd_str: &str, mut cmd: Command) -> Result<ExitStatus> {
    let mut child = handle_fs_res(&[&cmd_str], cmd.spawn())?;
    handle_fs_res(&[&cmd_str], child.wait())
}
#[cfg(not(target_os = "linux"))]
pub fn create_cmd(cmd_str: &str, args: &[impl AsRef<OsStr>]) -> Command {
    log::debug!("在非 linux 上執行，用 sh -c 包一層");
    let args: Vec<&str> = args.iter().map(|s| s.as_ref().to_str().unwrap()).collect();
    let arg = format!("{} {}", cmd_str, args.join(" ")).replace("/", "//");
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
pub fn fast_write_script(path: &PathBuf, content: &str) -> Result<()> {
    write_file(path, content)
}
pub fn remove(script: &ScriptMeta) -> Result<()> {
    handle_fs_res(&[&script.path], remove_file(&script.path))
}
pub fn mv(origin: &ScriptMeta, new: &ScriptMeta) -> Result<()> {
    handle_fs_res(&[&origin.path, &new.path], rename(&origin.path, &new.path))
}
pub fn cp(origin: &ScriptMeta, new: &ScriptMeta) -> Result<()> {
    let _copied = handle_fs_res(
        &[&origin.path, &new.path],
        std::fs::copy(&origin.path, &new.path),
    )?;
    Ok(())
}

pub fn handle_fs_err<P: AsRef<Path>>(path: &[P], err: std::io::Error) -> Error {
    let p = path.iter().map(|p| p.as_ref().to_owned()).collect();
    match err.kind() {
        std::io::ErrorKind::PermissionDenied => Error::PermissionDenied(p),
        std::io::ErrorKind::NotFound => Error::PathNotFound(path[0].as_ref().to_owned()),
        _ => Error::GeneralFS(p, err),
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
    content: Option<&str>,
) -> Result<Option<DateTime<Utc>>> {
    log::info!("開始準備 {:?} 腳本內容……", script);
    let mut is_new = if path.exists() {
        log::debug!("腳本已存在，不填入預設訊息");
        false
    } else {
        let birthplace = handle_fs_res(&["."], std::env::current_dir())?;
        let birthplace = birthplace.to_str().unwrap_or_default();
        let file = handle_fs_res(&[path], File::create(&path))?;
        let info = json!({
            "birthplace": birthplace,
            "name": script.name.key().to_owned(),
            "content": content.unwrap_or_default()
        });
        handle_fs_res(&[path], write_prepare_script(file, script, &info))?;
        true
    };
    if content.is_some() {
        is_new = false;
    }
    Ok(if is_new { Some(Utc::now()) } else { None })
}
fn write_prepare_script<W: Write>(
    w: W,
    script: &ScriptInfo,
    info: &serde_json::Value,
) -> std::io::Result<()> {
    // TODO: 依 ty 不同給不同訊息
    let template = match script.ty {
        ScriptType::Shell => templates::SHELL_WELCOME_MSG,
        ScriptType::Js => templates::JS_WELCOME_MSG,
        ScriptType::Tmux => templates::TMUX_WELCOME_MSG,
        _ => return Ok(()),
    };
    let reg = Handlebars::new();
    reg.render_template_to_write(template, &info, w)
        .map_err(|err| match err {
            TemplateRenderError::IOError(err, ..) => err,
            e => panic!("解析模版錯誤：{}", e),
        })
}
pub fn after_script(path: &Path, created: Option<DateTime<Utc>>) -> Result<()> {
    if let Some(created) = created {
        let meta = handle_fs_res(&[path], std::fs::metadata(path))?;
        let modified = handle_fs_res(&[path], meta.modified())?;
        let modified = modified.duration_since(std::time::UNIX_EPOCH)?.as_secs();
        if created.timestamp() >= modified as i64 {
            log::info!("腳本未變動，刪除之");
            handle_fs_res(&[path], remove_file(path))
        } else {
            Ok(())
        }
    } else {
        log::debug!("既存腳本，不執行後處理");
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_prepare() {
        use ScriptType::*;
        let test = &[Js, Shell, Tmux, Txt, Rb];
        for ty in test {
            let mut w = Vec::<u8>::new();
            // write_prepare_script(&mut w, *ty, "test_dir").expect("寫到 Vec<u8> 也能出事？");
        }
    }
}
