use crate::error::{Contextabl, Error, Result};
use crate::script::{Script, ScriptMeta, ScriptName, ScriptType, ToScriptName};
use crate::util::handle_fs_err;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::collections::HashMap;
use std::fs::{canonicalize, create_dir, read_dir, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const ANONYMOUS: &'static str = ".anonymous";
const META: &'static str = ".instant_script_meta.json";

lazy_static::lazy_static! {
    static ref PATH: Mutex<PathBuf> = Mutex::new(PathBuf::new());
}

#[cfg(not(debug_assertions))]
pub fn get_sys_path() -> Result<String> {
    let p = match std::env::var("INSTANT_SCRIPT_PATH") {
        Ok(p) => p,
        Err(std::env::VarError::NotPresent) => return Err(Error::PathNotSet),
        Err(e) => return Err(e.into()),
    };
    Ok(p)
}
#[cfg(debug_assertions)]
pub fn get_sys_path() -> Result<String> {
    Ok("./.instant_script".to_string())
}

pub fn join_path<B: AsRef<Path>, P: AsRef<Path>>(base: B, path: P) -> Result<PathBuf> {
    let here = canonicalize(base)?;
    Ok(here.join(path))
}

pub fn set_path<T: AsRef<Path>>(p: T) -> Result<()> {
    let path = join_path(".", p)?;
    if !path.exists() {
        return Err(Error::PathNotFound(path));
    }
    *PATH.lock().unwrap() = path;
    Ok(())
}
pub fn get_path() -> PathBuf {
    PATH.lock().unwrap().clone()
}

fn get_anonymous_ids() -> Result<Vec<u32>> {
    let mut ids = vec![];
    let dir = get_path().join(ANONYMOUS);
    if !dir.exists() {
        log::info!("找不到匿名腳本資料夾，創建之");
        handle_fs_err(&dir, create_dir(&dir))?;
    }
    for entry in handle_fs_err(&dir, read_dir(&dir))? {
        let mut name = entry?
            .file_name()
            .to_str()
            .ok_or(Error::msg("檔案實體為空...?"))?
            .to_string();
        let re = regex::Regex::new(r"\.\w+$").unwrap();
        let name = re.replace(&name, "");
        match name.parse::<u32>() {
            Ok(id) => ids.push(id),
            _ => log::info!("匿名腳本名無法轉為整數：{}", name),
        }
    }

    Ok(ids)
}
pub fn open_anonymous_script(id: Option<u32>, ty: ScriptType, read_only: bool) -> Result<Script> {
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
    let path = join_path(dir, &name.to_file_name(ty))?;
    let exist = path.exists();
    if read_only && !exist {
        return Err(Error::FileNotFound(path));
    }
    Ok(Script { path, exist, name })
}

pub fn open_script<T: ToScriptName>(name: T, ty: ScriptType, read_only: bool) -> Result<Script> {
    match name.to_script_name()? {
        ScriptName::Anonymous(id) => open_anonymous_script(Some(id), ty, read_only),
        ScriptName::Named(name) => {
            let name = ScriptName::Named(name);
            let path = join_path(get_path(), &name.to_file_name(ty))?;
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
                log::info!("找不到歷史檔案，視為空歷史");
                return Ok(map);
            } else {
                return handle_fs_err(&path, Err(e)).context("唯讀打開歷史檔案失敗");
            }
        }
    };
    let mut content = String::new();
    handle_fs_err(&path, file.read_to_string(&mut content)).context("讀取歷史檔案失敗")?;
    let histories: Vec<ScriptMeta> = serde_json::from_str(&content)?;
    for h in histories.into_iter() {
        match open_script(h.name.clone(), h.ty, true) {
            Err(e) => {
                log::warn!("{:?} 腳本歷史資料有誤：{:?}", h.name, e);
                continue;
            }
            _ => (),
        }
        map.insert(h.name.clone(), h);
    }
    Ok(map)
}

pub fn store_history(history: impl IntoIterator<Item = ScriptMeta>) -> Result<()> {
    let path = join_path(get_path(), META)?;
    let mut file = handle_fs_err(&path, File::create(&path)).context("唯讀打開歷史檔案失敗")?;
    let v: Vec<_> = history.into_iter().collect();
    handle_fs_err(&path, file.write_all(serde_json::to_string(&v)?.as_bytes()))
        .context("寫入歷史檔案失敗")?;
    Ok(())
}
#[cfg(test)]
mod test {
    use super::*;
    fn setup() {
        set_path(join_path(".", "./.test_instant_script").unwrap()).unwrap();
    }
    #[test]
    fn test_anonymous_ids() {
        setup();
        let mut ids = get_anonymous_ids().unwrap();
        ids.sort();
        assert_eq!(ids, vec![1, 2, 5]);
    }
    #[test]
    fn test_open_anonymous() {
        setup();
        let s = open_anonymous_script(None, ScriptType::Shell, false).unwrap();
        assert_eq!(s.name, ScriptName::Anonymous(6));
        assert_eq!(
            s.path,
            join_path("./.test_instant_script/.anonymous", "6.sh").unwrap()
        );
        let s = open_anonymous_script(None, ScriptType::Js, true).unwrap();
        assert_eq!(s.name, ScriptName::Anonymous(5));
        assert_eq!(
            s.path,
            join_path("./.test_instant_script/.anonymous", "5.js").unwrap()
        );
    }
    #[test]
    fn test_open() {
        setup();
        let s = open_script("first".to_owned(), ScriptType::Txt, false).unwrap();
        assert_eq!(s.name, ScriptName::Named("first".to_owned()));
        assert_eq!(s.exist, true);
        assert_eq!(
            s.path,
            join_path("./.test_instant_script/", "first").unwrap()
        );
        match open_script("not-exist".to_owned(), ScriptType::Shell, true) {
            Err(Error::FileNotFound(name)) => assert_eq!(
                name,
                join_path("./.test_instant_script/", "not-exist.sh").unwrap()
            ),
            _ => unreachable!(),
        }
    }
}
