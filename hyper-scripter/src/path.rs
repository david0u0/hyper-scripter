use crate::error::{Contextable, Error, Result};
use crate::script::{ScriptName, ANONYMOUS};
use crate::script_type::ScriptType;
use crate::util::{handle_fs_res, read_file};
use std::fs::{canonicalize, create_dir, read_dir};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const HS_EXECUTABLE_INFO_PATH: &'static str = ".hs_exe_path";
pub const HS_REDIRECT: &'static str = ".hs_redirect";

lazy_static::lazy_static! {
    static ref PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
}

#[cfg(not(debug_assertions))]
pub fn get_sys_home() -> Result<PathBuf> {
    use crate::error::SysPath;
    const ROOT_PATH: &'static str = "hyper_scripter";
    const HS_HOME_ENV: &'static str = "HYPER_SCRIPTER_HOME";

    let p = match std::env::var(HS_HOME_ENV) {
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
pub fn get_sys_home() -> Result<PathBuf> {
    Ok(".hyper_scripter".into())
}
#[cfg(all(debug_assertions, test))]
pub fn get_sys_home() -> Result<PathBuf> {
    Ok(".test_hyper_scripter".into())
}

fn join_path<B: AsRef<Path>, P: AsRef<Path>>(base: B, path: P) -> Result<PathBuf> {
    let here = canonicalize(base)?;
    Ok(here.join(path))
}

pub fn set_home_from_sys() -> Result {
    set_home(get_sys_home()?)
}
pub fn set_home<T: AsRef<Path>>(p: T) -> Result {
    let path = join_path(".", p)?;
    log::debug!("使用路徑：{:?}", path);
    if !path.exists() {
        log::info!("路徑 {:?} 不存在，嘗試創建之", path);
        handle_fs_res(&[&path], create_dir(&path))?;
    } else {
        let redirect = path.join(HS_REDIRECT);
        if redirect.is_file() {
            let redirect = read_file(&redirect)?;
            let redirect = redirect.trim();
            log::info!("重導向至 {}", redirect);
            return set_home(redirect);
        }
    }
    *PATH.lock().unwrap() = Some(path);
    Ok(())
}
#[cfg(not(test))]
pub fn get_home() -> PathBuf {
    PATH.lock()
        .unwrap()
        .clone()
        .expect("還沒設定路徑就取路徑，錯誤實作！")
}
#[cfg(test)]
pub fn get_home() -> PathBuf {
    join_path(".", ".test_hyper_scripter").unwrap()
}

fn get_anonymous_ids() -> Result<Vec<u32>> {
    let mut ids = vec![];
    let dir = get_home().join(ANONYMOUS);
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
pub fn open_new_anonymous(ty: &ScriptType) -> Result<(ScriptName, PathBuf)> {
    let ids = get_anonymous_ids().context("無法取得匿名腳本編號")?;
    let id = ids.into_iter().max().unwrap_or_default() + 1;
    Ok((ScriptName::Anonymous(id), open_anonymous(id, ty)?))
}
pub fn open_anonymous(id: u32, ty: &ScriptType) -> Result<PathBuf> {
    let name = ScriptName::Anonymous(id);
    let path = get_home().join(name.to_file_path(ty)?);
    Ok(path)
}

pub fn open_script(
    name: &ScriptName,
    ty: &ScriptType,
    check_sxist: Option<bool>,
) -> Result<PathBuf> {
    let script_path = match &name {
        ScriptName::Anonymous(id) => open_anonymous(*id, ty)?,
        ScriptName::Named(_) => {
            let path = get_home().join(name.to_file_path(ty)?);
            path
        }
    };
    if let Some(should_exist) = check_sxist {
        if !script_path.exists() && should_exist {
            return Err(
                Error::PathNotFound(vec![script_path]).context("開腳本失敗：應存在卻不存在")
            );
        } else if script_path.exists() && !should_exist {
            return Err(Error::PathExist(script_path).context("開腳本失敗：不應存在卻存在"));
        }
    }
    Ok(script_path)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::script::IntoScriptName;
    #[test]
    fn test_anonymous_ids() {
        let mut ids = get_anonymous_ids().unwrap();
        ids.sort();
        assert_eq!(ids, vec![1, 2, 5]);
    }
    #[test]
    fn test_open_anonymous() {
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
        let second = "second".to_owned();
        let second_name = second.to_owned().into_script_name().unwrap();
        let p = open_script(&second_name, &"rb".into(), Some(false)).unwrap();
        assert_eq!(p, get_home().join("second.rb"));

        let p = open_script(
            &".1".to_owned().into_script_name().unwrap(),
            &"sh".into(),
            None,
        )
        .unwrap();
        assert_eq!(
            p,
            join_path("./.test_hyper_scripter/.anonymous", "1.sh").unwrap()
        );

        match open_script(
            &"not-exist".to_owned().into_script_name().unwrap(),
            &"sh".into(),
            Some(true),
        )
        .unwrap_err()
        {
            Error::PathNotFound(name) => assert_eq!(name[0], get_home().join("not-exist.sh")),
            _ => unreachable!(),
        }
    }
}
