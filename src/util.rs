use crate::error::{Contextabl, Error, Result};
use crate::script::{ScriptMeta, ScriptType};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run(script: &ScriptMeta, ty: ScriptType, remaining: &[String]) -> Result<()> {
    let (cmd_str, args) = ty
        .cmd()
        .ok_or(Error::Operation(format!("{} is not runnable", ty)))?;
    let mut cmd = Command::new(&cmd_str);
    let mut full_args: Vec<PathBuf> = args.into_iter().map(|s| s.into()).collect();
    full_args.push(script.path.clone());
    full_args.extend(remaining.into_iter().map(|s| s.into()));
    let mut child = handle_fs_err(&[&cmd_str], cmd.args(full_args).spawn())?;
    // TODO: 看要不要把執行狀態傳回去？
    let stat = handle_fs_err(&[&cmd_str], child.wait())?;
    log::info!("程式執行結果：{:?}", stat);
    Ok(())
}
pub fn read_file(path: &PathBuf) -> Result<String> {
    let mut file = handle_fs_err(&[path], std::fs::File::open(path)).context("唯讀開啟檔案失敗")?;
    let mut content = String::new();
    handle_fs_err(&[path], file.read_to_string(&mut content)).context("讀取檔案失敗")?;
    Ok(content)
}

pub fn remove(script: &ScriptMeta) -> Result<()> {
    handle_fs_err(&[&script.path], std::fs::remove_file(&script.path))
}
pub fn mv(origin: &ScriptMeta, new: &ScriptMeta) -> Result<()> {
    handle_fs_err(
        &[&origin.path, &new.path],
        std::fs::rename(&origin.path, &new.path),
    )
}
pub fn map_to_iter<K, V>(map: HashMap<K, V>) -> impl IntoIterator<Item = V> {
    map.into_iter().map(|(_, v)| v)
}

pub fn handle_fs_err<T, P: AsRef<Path>>(path: &[P], res: std::io::Result<T>) -> Result<T> {
    match res {
        Ok(t) => Ok(t),
        Err(e) => {
            let p = path.iter().map(|p| p.as_ref().to_owned()).collect();
            match e.kind() {
                std::io::ErrorKind::PermissionDenied => Err(Error::PermissionDenied(p)),
                std::io::ErrorKind::NotFound => Err(Error::FileNotFound(p)),
                _ => Err(Error::GeneralFS(p, e)),
            }
        }
    }
}
