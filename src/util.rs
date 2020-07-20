use crate::error::{Contextabl, Error, Result};
use crate::script::{CommandType, ScriptMeta};
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;

pub fn run(script: &ScriptMeta, ty: CommandType, remaining: &[String]) -> Result<()> {
    let (cmd, args) = ty
        .cmd()
        .ok_or(Error::Operation(format!("{} is not runnable", ty)))?;
    let mut cmd = Command::new(cmd);
    let mut full_args: Vec<PathBuf> = args.into_iter().map(|s| s.into()).collect();
    full_args.push(script.path.clone());
    full_args.extend(remaining.into_iter().map(|s| s.into()));
    cmd.args(full_args).spawn()?.wait()?;
    Ok(())
}
pub fn read_file(path: &PathBuf) -> Result<String> {
    let mut file = handle_fs_err(path, std::fs::File::open(path)).context("唯讀開啟檔案失敗")?;
    let mut content = String::new();
    handle_fs_err(path, file.read_to_string(&mut content)).context("讀取檔案失敗")?;
    Ok(content)
}

pub fn remove(script: &ScriptMeta) -> Result<()> {
    handle_fs_err(&script.path, std::fs::remove_file(&script.path))
}
pub fn map_to_iter<K, V>(map: HashMap<K, V>) -> impl IntoIterator<Item = V> {
    map.into_iter().map(|(_, v)| v)
}

pub fn handle_fs_err<T>(path: &PathBuf, res: std::io::Result<T>) -> Result<T> {
    match res {
        Ok(t) => Ok(t),
        Err(e) => match e.kind() {
            std::io::ErrorKind::PermissionDenied => Err(Error::PermissionDenied(path.clone())),
            std::io::ErrorKind::NotFound => Err(Error::FileNotFound(path.clone())),
            _ => Err(Error::GeneralFS(path.clone(), e)),
        },
    }
}
