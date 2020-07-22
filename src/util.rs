use crate::error::{Contextabl, Error, Result};
use crate::script::{ScriptInfo, ScriptMeta};
use std::fs::{remove_file, rename, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const IS_BIRTH_PLACE: &'static str = "IS_BIRTH_PLACE";

pub fn run(script: &ScriptMeta, info: &ScriptInfo, remaining: &[String]) -> Result<()> {
    let ty = info.ty;
    let (cmd_str, args) = ty
        .cmd()
        .ok_or(Error::Operation(format!("{} is not runnable", ty)))?;
    let mut cmd = Command::new(&cmd_str);
    let mut full_args: Vec<PathBuf> = args.into_iter().map(|s| s.into()).collect();
    full_args.push(script.path.clone());
    full_args.extend(remaining.into_iter().map(|s| s.into()));
    cmd.args(full_args).env(IS_BIRTH_PLACE, &info.birthplace);
    let mut child = handle_fs_err(&[&cmd_str], cmd.spawn())?;
    // TODO: 看要不要把執行狀態傳回去？
    let stat = handle_fs_err(&[&cmd_str], child.wait())?;
    log::info!("程式執行結果：{:?}", stat);
    Ok(())
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
