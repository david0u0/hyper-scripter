use crate::error::{Error, Result};
use crate::script::{Script, ScriptType};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

pub fn run(script: &Script, ty: ScriptType, remaining: &[String]) -> Result<()> {
    let (cmd, args) = ty
        .cmd()
        .ok_or(Error::Operation(format!("{} is not runnable", ty)))?;
    let mut cmd = Command::new(cmd);
    let mut full_args: Vec<PathBuf> = args.into_iter().map(|s| s.into()).collect();
    full_args.extend(remaining.into_iter().map(|s| s.into()));
    full_args.push(script.path.clone());
    cmd.args(full_args).spawn()?.wait()?;
    Ok(())
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
