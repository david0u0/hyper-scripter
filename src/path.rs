use crate::error::{Contextabl, Error, Result};
use crate::script::{Script, ScriptMeta, ScriptName, ToScriptName};
use std::collections::HashMap;
use std::fs::{canonicalize, read_dir, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const ANONYMOUS: &'static str = ".anonymous";
const META: &'static str = ".flash_script_meta.json";

lazy_static::lazy_static! {
    static ref PATH: Mutex<PathBuf> = Mutex::new(join_path(".", &get_sys_path()).unwrap());
}

#[cfg(release)]
fn get_sys_path() -> String {}
#[cfg(all(not(release), not(test)))]
fn get_sys_path() -> String {
    "./.flash_script".to_string()
}
#[cfg(test)]
fn get_sys_path() -> String {
    "./.test_flash_script".to_string()
}

fn join_path<P: AsRef<Path>>(base: P, path: &str) -> Result<PathBuf> {
    let here = canonicalize(base)?;
    Ok(here.join(Path::new(path)))
}

pub fn set_path<T: AsRef<str>>(p: T) -> Result<()> {
    let path = join_path(".", p.as_ref())?;
    if !path.exists() {
        return Err(Error::PathNotFound(path));
    }
    *PATH.lock().unwrap() = path;
    Ok(())
}
pub fn get_path() -> PathBuf {
    PATH.lock().unwrap().clone()
}

pub fn get_anonymous_ids() -> Result<Vec<u32>> {
    let mut ids = vec![];
    let dir = get_path().join(ANONYMOUS);
    for entry in read_dir(dir)? {
        let mut name = entry?
            .file_name()
            .to_str()
            .ok_or(Error::msg("檔案實體為空...?"))?
            .to_string();
        name = name.replace(".sh", "");
        match name.parse::<u32>() {
            Ok(id) => ids.push(id),
            _ => log::info!("匿名腳本名無法轉為整數：{}", name),
        }
    }

    Ok(ids)
}
pub fn open_anonymous_script(id: Option<u32>, read_only: bool) -> Result<Script> {
    let ids = get_anonymous_ids().context("無法取得匿名腳本編號")?;
    let dir = get_path().join(ANONYMOUS);
    let actual_id = if let Some(id) = id {
        id
    } else {
        match (read_only, ids.into_iter().max()) {
            (true, None) => return Err(Error::Empty),
            (true, Some(id)) => id,
            (_, t) => t.unwrap_or_default() + 1,
        }
    };

    let name = ScriptName::Anonymous(actual_id);
    let path = join_path(dir, &name.to_cmd())?;
    let exist = path.exists();
    if read_only && !exist {
        return Err(Error::FileNotFound(path));
    }
    Ok(Script { path, exist, name })
}

pub fn open_script<T: ToScriptName>(name: T, read_only: bool) -> Result<Script> {
    match name.to_script_name()? {
        ScriptName::Anonymous(id) => open_anonymous_script(Some(id), read_only),
        ScriptName::Named(name) => {
            let name = ScriptName::Named(name);
            let path = join_path(get_path(), &name.to_cmd())?;
            let exist = path.exists();
            if read_only && !exist {
                return Err(Error::FileNotFound(path));
            }
            Ok(Script { exist, path, name })
        }
    }
}
pub fn get_history() -> Result<HashMap<ScriptName, ScriptMeta>> {
    let path = join_path(get_path(), META)?;
    let mut map = HashMap::new();
    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Ok(map);
            } else {
                return Err(e.into());
            }
        }
    };
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let histories: Vec<ScriptMeta> = serde_json::from_str(&content)?;
    for h in histories.into_iter() {
        map.insert(h.name.clone(), h);
    }
    Ok(map)
}

pub fn store_history(history: impl IntoIterator<Item = ScriptMeta>) -> Result<()> {
    let path = join_path(get_path(), META)?;
    let mut file = File::create(path)?;
    let v: Vec<_> = history.into_iter().collect();
    file.write_all(serde_json::to_string(&v)?.as_bytes())?;
    Ok(())
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_anonymous_ids() {
        let mut ids = get_anonymous_ids().unwrap();
        ids.sort();
        assert_eq!(ids, vec![1, 2, 5]);
    }
    #[test]
    fn test_open_anonymous() {
        let s = open_anonymous_script(None, false).unwrap();
        assert_eq!(s.name, ScriptName::Anonymous(6));
        assert_eq!(
            s.path,
            join_path("./.test_flash_script/.anonymous", "6.sh").unwrap()
        );
        let s = open_anonymous_script(None, true).unwrap();
        assert_eq!(s.name, ScriptName::Anonymous(5));
        assert_eq!(
            s.path,
            join_path("./.test_flash_script/.anonymous", "5.sh").unwrap()
        );
    }
    #[test]
    fn test_open() {
        let s = open_script("first".to_owned(), false).unwrap();
        assert_eq!(s.name, ScriptName::Named("first".to_owned()));
        assert_eq!(s.exist, true);
        assert_eq!(
            s.path,
            join_path("./.test_flash_script/", "first.sh").unwrap()
        );
        match open_script("not-exist".to_owned(), true) {
            Err(Error::FileNotFound(name)) => assert_eq!(
                name,
                join_path("./.test_flash_script/", "not-exist.sh").unwrap()
            ),
            _ => unreachable!(),
        }
    }
}
