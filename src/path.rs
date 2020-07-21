use crate::error::{Contextabl, Error, Result};
use crate::script::{ScriptInfo, ScriptMeta, ScriptName, ScriptType, ToScriptName, ANONYMOUS};
use crate::util::{handle_fs_err, read_file};
use std::collections::HashMap;
use std::fs::{canonicalize, create_dir, read_dir, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const META: &'static str = ".instant_scripter_info.json";
const ROOT_PATH: &'static str = "instant_scripter";

lazy_static::lazy_static! {
    static ref PATH: Mutex<PathBuf> = Mutex::new(PathBuf::new());
}

#[cfg(not(debug_assertions))]
pub fn get_sys_path() -> Result<PathBuf> {
    let p = match std::env::var("INSTANT_SCRIPT_PATH") {
        Ok(p) => {
            log::debug!("使用環境變數路徑：{}", p);
            p.into()
        }
        Err(std::env::VarError::NotPresent) => dirs::config_dir()
            .ok_or(Error::SysPathNotFound("config"))?
            .join(ROOT_PATH),
        Err(e) => return Err(e.into()),
    };
    log::debug!("使用路徑：{:?}", p);
    Ok(p)
}
#[cfg(debug_assertions)]
pub fn get_sys_path() -> Result<PathBuf> {
    Ok("./.instant_script".into())
}

pub fn join_path<B: AsRef<Path>, P: AsRef<Path>>(base: B, path: P) -> Result<PathBuf> {
    let here = canonicalize(base)?;
    Ok(here.join(path))
}

pub fn set_path<T: AsRef<Path>>(p: T) -> Result<()> {
    let path = join_path(".", p)?;
    if !path.exists() {
        log::info!("路徑 {:?} 不存在，嘗試創建之", path);
        handle_fs_err(&[&path], create_dir(&path))?;
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
        handle_fs_err(&[&dir], create_dir(&dir))?;
    }
    for entry in handle_fs_err(&[&dir], read_dir(&dir))? {
        let name = entry?
            .file_name()
            .to_str()
            .ok_or(Error::msg("檔案實體為空...?"))?
            .to_string();
        let re = regex::Regex::new(r"\.\w+$").unwrap();
        let name = re.replace(&name, "");
        match name.parse::<u32>() {
            Ok(id) => ids.push(id),
            _ => log::warn!("匿名腳本名無法轉為整數：{}", name),
        }
    }

    Ok(ids)
}
pub fn open_new_anonymous(ty: ScriptType) -> Result<ScriptMeta> {
    let ids = get_anonymous_ids().context("無法取得匿名腳本編號")?;
    let id = ids.into_iter().max().unwrap_or_default() + 1;
    open_anonymous(id, ty)
}
pub fn open_anonymous(id: u32, ty: ScriptType) -> Result<ScriptMeta> {
    let name = ScriptName::Anonymous(id);
    let path = get_path().join(name.to_file_name(ty));
    Ok(ScriptMeta { path, name })
}

pub fn open_script<T: ToScriptName>(
    name: T,
    ty: ScriptType,
    check_sxist: bool,
) -> Result<ScriptMeta> {
    let script = match name.to_script_name()? {
        ScriptName::Anonymous(id) => open_anonymous(id, ty)?,
        ScriptName::Named(name) => {
            let name = ScriptName::Named(name);
            let path = get_path().join(name.to_file_name(ty));
            ScriptMeta { path, name }
        }
    };
    if check_sxist && !script.path.exists() {
        Err(Error::PathNotFound(vec![script.path]))
    } else {
        Ok(script)
    }
}
pub fn get_history() -> Result<HashMap<ScriptName, ScriptInfo>> {
    let path = join_path(get_path(), META)?;
    let mut map = HashMap::new();
    let content = match read_file(&path) {
        Ok(s) => s,
        Err(Error::PathNotFound(_)) => {
            log::info!("找不到歷史檔案，視為空歷史");
            return Ok(map);
        }
        Err(e) => return Err(e).context("打開歷史檔案失敗"),
    };
    let histories: Vec<ScriptInfo> = serde_json::from_str(&content)?;
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

pub fn store_history(history: impl IntoIterator<Item = ScriptInfo>) -> Result<()> {
    let path = join_path(get_path(), META)?;
    let mut file = handle_fs_err(&[&path], File::create(&path)).context("唯寫打開歷史檔案失敗")?;
    let v: Vec<_> = history.into_iter().collect();
    handle_fs_err(
        &[&path],
        file.write_all(serde_json::to_string(&v)?.as_bytes()),
    )
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
        let s = open_new_anonymous(ScriptType::Shell).unwrap();
        assert_eq!(s.name, ScriptName::Anonymous(6));
        assert_eq!(
            s.path,
            join_path("./.test_instant_script/.anonymous", "6.sh").unwrap()
        );
        let s = open_anonymous(5, ScriptType::Js).unwrap();
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

        let s = open_script(".1".to_owned(), ScriptType::Rb, false).unwrap();
        assert_eq!(s.name, ScriptName::Anonymous(1));
        assert_eq!(
            s.path,
            join_path("./.test_instant_script/.anonymous", "1.rb").unwrap()
        );

        match open_script("not-exist".to_owned(), ScriptType::Shell, true) {
            Err(Error::PathNotFound(name)) => assert_eq!(
                name,
                vec![join_path("./.test_instant_script/", "not-exist.sh").unwrap()]
            ),
            _ => unreachable!(),
        }
    }
}
