use crate::error::{Contextabl, Error, Result};
use crate::script::{ScriptInfo, ScriptMeta, ScriptType};
use crate::templates;
use mustache::{compile_str, MapBuilder};
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

pub fn fast_write_script(script: &ScriptMeta, content: &str) -> Result<()> {
    let mut file = handle_fs_res(&[&script.path], File::create(&script.path))?;
    handle_fs_res(&[&script.path], file.write_all(content.as_bytes()))
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

pub fn prepare_script(path: &Path, ty: ScriptType) -> Result<()> {
    if path.exists() {
        log::debug!("腳本已存在，不填入內容");
        return Ok(());
    }
    log::debug!("開始準備 {} 腳本內容……", ty);
    let mut file = handle_fs_res(&[path], File::create(&path))?;
    let birthplace = handle_fs_res(&["."], std::env::current_dir())?;
    let birthplace = birthplace.to_str().unwrap_or_default();
    let data = MapBuilder::new()
        .insert_str("birthplace", birthplace)
        .build();
    // TODO: 依 ty 不同給不同訊息
    let template = match ty {
        ScriptType::Shell => templates::SHELL_WELCOME_MSG,
        ScriptType::Js => templates::JS_WELCOME_MSG,
        _ => return Ok(()),
    };
    compile_str(template)
        .unwrap()
        .render_data(&mut file, &data)
        .map_err(|e| match e {
            mustache::Error::Io(e) => handle_fs_err(&[path], e),
            mustache::Error::Parser(e) => panic!("模版解析失敗 {}", e),
            _ => e.into(),
        })
}
