use crate::error::{Contextabl, Error, Result};
use crate::history::History;
use crate::script::{AsScriptName, ScriptInfo, ScriptMeta, ScriptName, ANONYMOUS};
use crate::script_type::ScriptType;
use crate::util::{handle_fs_res, read_file, write_file};
use std::fs::{canonicalize, create_dir, read_dir};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const META: &'static str = ".instant_scripter_info.json";
const ROOT_PATH: &'static str = "instant_scripter";

lazy_static::lazy_static! {
    static ref PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
}

#[cfg(not(debug_assertions))]
pub fn get_sys_path() -> Result<PathBuf> {
    let p = match std::env::var("INSTANT_SCRIPT_PATH") {
        Ok(p) => {
            log::debug!("使用環境變數路徑：{}", p);
            p.into()
        }
        Err(std::env::VarError::NotPresent) => dirs::config_dir()
            .ok_or(Error::SysPathNotFound("config dir"))?
            .join(ROOT_PATH),
        Err(e) => return Err(e.into()),
    };
    Ok(p)
}
#[cfg(all(debug_assertions, not(test)))]
pub fn get_sys_path() -> Result<PathBuf> {
    Ok(".instant_script".into())
}
#[cfg(all(debug_assertions, test))]
pub fn get_sys_path() -> Result<PathBuf> {
    Ok(".test_instant_script".into())
}

pub fn join_path<B: AsRef<Path>, P: AsRef<Path>>(base: B, path: P) -> Result<PathBuf> {
    let here = canonicalize(base)?;
    Ok(here.join(path))
}

pub fn set_path_from_sys() -> Result<()> {
    set_path(get_sys_path()?)
}
pub fn set_path<T: AsRef<Path>>(p: T) -> Result<()> {
    let path = join_path(".", p)?;
    log::debug!("使用路徑：{:?}", path);
    if !path.exists() {
        log::info!("路徑 {:?} 不存在，嘗試創建之", path);
        handle_fs_res(&[&path], create_dir(&path))?;
    }
    *PATH.lock().unwrap() = Some(path);
    Ok(())
}
pub fn get_path() -> PathBuf {
    PATH.lock()
        .unwrap()
        .clone()
        .expect("還沒設定路徑就取路徑，錯誤實作！")
}

fn get_anonymous_ids() -> Result<Vec<u32>> {
    let mut ids = vec![];
    let dir = get_path().join(ANONYMOUS);
    if !dir.exists() {
        log::info!("找不到匿名腳本資料夾，創建之");
        handle_fs_res(&[&dir], create_dir(&dir))?;
    }
    for entry in handle_fs_res(&[&dir], read_dir(&dir))? {
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
pub fn open_new_anonymous(ty: &ScriptType) -> Result<ScriptMeta<'static>> {
    let ids = get_anonymous_ids().context("無法取得匿名腳本編號")?;
    let id = ids.into_iter().max().unwrap_or_default() + 1;
    open_anonymous(id, ty)
}
pub fn open_anonymous(id: u32, ty: &ScriptType) -> Result<ScriptMeta<'static>> {
    let name = ScriptName::Anonymous(id);
    let path = get_path().join(name.to_file_path(ty)?);
    Ok(ScriptMeta { path, name })
}

pub fn open_script<'a, T: ?Sized + AsScriptName>(
    name: &'a T,
    ty: &ScriptType,
    check_sxist: bool,
) -> Result<ScriptMeta<'a>> {
    let script = match name.as_script_name()? {
        ScriptName::Anonymous(id) => open_anonymous(id, ty)?,
        ScriptName::Named(name) => {
            let name = ScriptName::Named(name);
            let path = get_path().join(name.to_file_path(ty)?);
            ScriptMeta { path, name }
        }
    };
    if check_sxist && !script.path.exists() {
        Err(Error::PathNotFound(script.path))
    } else {
        Ok(script)
    }
}
pub fn get_history() -> Result<History<'static>> {
    let path = join_path(get_path(), META)?;
    let content = match read_file(&path) {
        Ok(s) => s,
        Err(Error::PathNotFound(_)) => {
            log::info!("找不到歷史檔案，視為空歷史");
            return Ok(Default::default());
        }
        Err(e) => return Err(e).context("打開歷史檔案失敗"),
    };
    let history: Vec<ScriptInfo> = serde_json::from_str(&content)?;
    let history =
        History::new(
            history
                .into_iter()
                .filter(|s| match open_script(&s.name, &s.ty, true) {
                    Err(e) => {
                        log::warn!("{:?} 腳本歷史資料有誤：{:?}", s.name, e);
                        false
                    }
                    _ => true,
                }),
        );
    Ok(history)
}

pub fn store_history<'a>(history: impl Iterator<Item = ScriptInfo<'a>>) -> Result<()> {
    let path = join_path(get_path(), META)?;
    let v: Vec<_> = history.collect();
    write_file(&path, &serde_json::to_string(&v)?).context("寫入歷史檔案失敗")?;
    Ok(())
}
#[cfg(test)]
mod test {
    use super::*;
    fn setup() {
        set_path_from_sys().unwrap();
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
        let s = open_new_anonymous(&"sh".into()).unwrap();
        assert_eq!(s.name, ScriptName::Anonymous(6));
        assert_eq!(
            s.path,
            join_path("./.test_instant_script/.anonymous", "6.sh").unwrap()
        );
        let s = open_anonymous(5, &"js".into()).unwrap();
        assert_eq!(s.name, ScriptName::Anonymous(5));
        assert_eq!(
            s.path,
            join_path("./.test_instant_script/.anonymous", "5.js").unwrap()
        );
    }
    #[test]
    fn test_open() {
        setup();
        let s = open_script("first", &"md".into(), true).unwrap();
        assert_eq!(s.name, "first".as_script_name().unwrap());

        let second = "second".to_owned();
        let second_name = second.as_script_name().unwrap();
        let s = open_script(&second_name, &"rb".into(), false).unwrap();
        assert_eq!(s.name, second_name);
        assert_eq!(s.path, get_path().join("second.rb"));

        let s = open_script(".1", &"sh".into(), true).unwrap();
        assert_eq!(s.name, ScriptName::Anonymous(1));
        assert_eq!(
            s.path,
            join_path("./.test_instant_script/.anonymous", "1.sh").unwrap()
        );

        match open_script("not-exist", &"sh".into(), true).unwrap_err() {
            Error::PathNotFound(name) => assert_eq!(name, get_path().join("not-exist.sh")),
            _ => unreachable!(),
        }
    }
}
