use crate::error::{Contextabl, Error, Result};
use crate::script::{ScriptInfo, ScriptMeta};
use std::ffi::OsStr;
use std::fs::{remove_file, rename, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

const IS_BIRTH_PLACE: &'static str = "IS_BIRTH_PLACE";

pub fn run(script: &ScriptMeta, info: &ScriptInfo, remaining: &[String]) -> Result<()> {
    let ty = info.ty;
    let (cmd_str, args) = ty
        .cmd()
        .ok_or(Error::Operation(format!("{} is not runnable", ty)))?;
    let mut full_args: Vec<&OsStr> = args.iter().map(|s| s.as_ref()).collect();

    full_args.push(script.path.as_ref());
    full_args.extend(remaining.iter().map(|s| AsRef::<OsStr>::as_ref(s)));
    // TODO: 看要不要把執行狀態傳回去？
    let mut cmd = create_cmd(&cmd_str, &full_args);
    cmd.env(IS_BIRTH_PLACE, &info.birthplace);
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
    let output = handle_fs_err(&[&cmd_str], cmd.output())?;
    println!("{}", std::str::from_utf8(&output.stdout)?);
    Ok(output.status)
}
#[cfg(target_os = "linux")]
pub fn run_cmd(cmd_str: &str, mut cmd: Command) -> Result<ExitStatus> {
    let mut child = handle_fs_err(&[&cmd_str], cmd.spawn())?;
    handle_fs_err(&[&cmd_str], child.wait())
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
    let mut file = handle_fs_err(&[path], File::open(path)).context("唯讀開啟檔案失敗")?;
    let mut content = String::new();
    handle_fs_err(&[path], file.read_to_string(&mut content)).context("讀取檔案失敗")?;
    Ok(content)
}

pub fn fast_write_script(script: &ScriptMeta, content: &str) -> Result<()> {
    let mut file = handle_fs_err(&[&script.path], File::create(&script.path))?;
    handle_fs_err(&[&script.path], file.write_all(content.as_bytes()))
}
pub fn remove(script: &ScriptMeta) -> Result<()> {
    handle_fs_err(&[&script.path], remove_file(&script.path))
}
pub fn mv(origin: &ScriptMeta, new: &ScriptMeta) -> Result<()> {
    handle_fs_err(&[&origin.path, &new.path], rename(&origin.path, &new.path))
}
pub fn cp(origin: &ScriptMeta, new: &ScriptMeta) -> Result<()> {
    let _copied = handle_fs_err(
        &[&origin.path, &new.path],
        std::fs::copy(&origin.path, &new.path),
    )?;
    Ok(())
}

pub fn handle_fs_err<T, P: AsRef<Path>>(path: &[P], res: std::io::Result<T>) -> Result<T> {
    match res {
        Ok(t) => Ok(t),
        Err(e) => {
            let p = path.iter().map(|p| p.as_ref().to_owned()).collect();
            match e.kind() {
                std::io::ErrorKind::PermissionDenied => Err(Error::PermissionDenied(p)),
                std::io::ErrorKind::NotFound => {
                    Err(Error::PathNotFound(path[0].as_ref().to_owned()))
                }
                _ => Err(Error::GeneralFS(p, e)),
            }
        }
    }
}
