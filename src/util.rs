use crate::error::{Error, Result};
use crate::script::Script;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

pub fn run(script: &Script, args: Vec<String>) -> Result<()> {
    let mut cmd = Command::new("sh");
    let mut full_args = vec![script.path.clone()];
    full_args.extend(args.into_iter().map(|s| s.into()));
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
