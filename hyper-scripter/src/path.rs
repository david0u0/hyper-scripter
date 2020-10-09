use crate::error::{Contextable, Error, Result, SysPath};
use crate::script::{AsScriptName, ScriptName, ANONYMOUS};
use crate::script_type::ScriptType;
use crate::util::handle_fs_res;
use std::fs::{canonicalize, create_dir, read_dir};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const ROOT_PATH: &'static str = "hyper_scripter";
pub const HS_EXECUTABLE_INFO_PATH: &'static str = ".hs_exe_path";

lazy_static::lazy_static! {
    static ref PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
}

#[cfg(not(debug_assertions))]
pub fn get_sys_path() -> Result<PathBuf> {
    let p = match std::env::var("HYPER_SCRIPTER_PATH") {
        Ok(p) => {
            log::debug!("使用環境變數路徑：{}", p);
            p.into()
        }
        Err(std::env::VarError::NotPresent) => dirs::config_dir()
            .ok_or(Error::SysPathNotFound(SysPath::Config))?
            .join(ROOT_PATH),
        Err(e) => return Err(e.into()),
    };
    Ok(p)
}
#[cfg(all(debug_assertions, not(test)))]
pub fn get_sys_path() -> Result<PathBuf> {
    Ok(".hyper_scripter".into())
}
#[cfg(all(debug_assertions, test))]
pub fn get_sys_path() -> Result<PathBuf> {
    Ok(".test_hyper_scripter".into())
}

fn join_path<B: AsRef<Path>, P: AsRef<Path>>(base: B, path: P) -> Result<PathBuf> {
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
pub fn open_new_anonymous(ty: &ScriptType) -> Result<(ScriptName<'static>, PathBuf)> {
    let ids = get_anonymous_ids().context("無法取得匿名腳本編號")?;
    let id = ids.into_iter().max().unwrap_or_default() + 1;
    Ok((ScriptName::Anonymous(id), open_anonymous(id, ty)?))
}
pub fn open_anonymous(id: u32, ty: &ScriptType) -> Result<PathBuf> {
    let name = ScriptName::Anonymous(id);
    let path = get_path().join(name.to_file_path(ty)?);
    Ok(path)
}

pub fn open_script<T: ?Sized + AsScriptName>(
    name: &T,
    ty: &ScriptType,
    check_sxist: bool,
) -> Result<PathBuf> {
    let script_path = match name.as_script_name()? {
        ScriptName::Anonymous(id) => open_anonymous(id, ty)?,
        ScriptName::Named(name) => {
            let name = ScriptName::Named(name);
            let path = get_path().join(name.to_file_path(ty)?);
            path
        }
    };
    if check_sxist && !script_path.exists() {
        Err(Error::PathNotFound(script_path))
    } else {
        Ok(script_path)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    fn setup() {
        set_path(".test_hyper_scripter").unwrap();
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
        let (name, p) = open_new_anonymous(&"sh".into()).unwrap();
        assert_eq!(name, ScriptName::Anonymous(6));
        assert_eq!(
            p,
            join_path("./.test_hyper_scripter/.anonymous", "6.sh").unwrap()
        );
        let p = open_anonymous(5, &"js".into()).unwrap();
        assert_eq!(
            p,
            join_path("./.test_hyper_scripter/.anonymous", "5.js").unwrap()
        );
    }
    #[test]
    fn test_open() {
        setup();
        let second = "second".to_owned();
        let second_name = second.as_script_name().unwrap();
        let p = open_script(&second_name, &"rb".into(), false).unwrap();
        assert_eq!(p, get_path().join("second.rb"));

        let p = open_script(".1", &"sh".into(), true).unwrap();
        assert_eq!(
            p,
            join_path("./.test_hyper_scripter/.anonymous", "1.sh").unwrap()
        );

        match open_script("not-exist", &"sh".into(), true).unwrap_err() {
            Error::PathNotFound(name) => assert_eq!(name, get_path().join("not-exist.sh")),
            _ => unreachable!(),
        }
    }
}
