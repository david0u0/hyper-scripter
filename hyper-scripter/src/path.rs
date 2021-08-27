use crate::error::{Contextable, Error, Result};
use crate::script::IntoScriptName;
use crate::script::{ScriptName, ANONYMOUS};
use crate::script_type::ScriptType;
use crate::state::State;
use crate::util::{handle_fs_res, read_file};
use std::fs::{canonicalize, create_dir, read_dir};
use std::path::{Path, PathBuf};

pub const HS_REDIRECT: &str = ".hs_redirect";
pub const HS_PRE_RUN: &str = ".hs_prerun.sh";
const TEMPLATE: &str = ".hs_templates";

macro_rules! hs_home_env {
    () => {
        "HYPER_SCRIPTER_HOME"
    };
}

static PATH: State<PathBuf> = State::new();

#[cfg(not(feature = "hard-home"))]
fn get_default_home() -> Result<PathBuf> {
    const ROOT_PATH: &str = "hyper_scripter";
    use crate::error::SysPath;
    let home = dirs::config_dir()
        .ok_or(Error::SysPathNotFound(SysPath::Config))?
        .join(ROOT_PATH);
    Ok(home)
}
#[cfg(feature = "hard-home")]
fn get_default_home() -> Result<PathBuf> {
    let home = env!(
        hs_home_env!(),
        concat!("Hardcoded home ", hs_home_env!(), " not provided!",)
    );
    Ok(home.into())
}

#[cfg(not(test))]
fn get_sys_home() -> Result<PathBuf> {
    let p = match std::env::var(hs_home_env!()) {
        Ok(p) => {
            log::debug!("使用環境變數路徑：{}", p);
            p.into()
        }
        Err(std::env::VarError::NotPresent) => get_default_home()?,
        Err(e) => return Err(e.into()),
    };
    Ok(p)
}
#[cfg(test)]
fn get_sys_home() -> Result<PathBuf> {
    Ok(".test_hyper_scripter".into())
}

fn join_path<B: AsRef<Path>, P: AsRef<Path>>(base: B, path: P) -> Result<PathBuf> {
    let path = path.as_ref();
    if path.is_absolute() {
        return Ok(path.to_owned());
    }
    let here = canonicalize(base)?;
    Ok(here.join(path))
}

fn compute_home_path<T: AsRef<Path>>(p: T) -> Result<PathBuf> {
    let path = join_path(".", p)?;
    log::debug!("計算路徑：{:?}", path);
    if !path.exists() {
        log::info!("路徑 {:?} 不存在，嘗試創建之", path);
        handle_fs_res(&[&path], create_dir(&path))?;
    } else {
        let redirect = path.join(HS_REDIRECT);
        if redirect.is_file() {
            let redirect = read_file(&redirect)?;
            let redirect = redirect.trim();
            log::info!("重導向至 {}", redirect);
            return compute_home_path(redirect);
        }
    }
    Ok(path)
}
pub fn compute_home_path_optional<T: AsRef<Path>>(p: Option<T>) -> Result<PathBuf> {
    match p {
        Some(p) => compute_home_path(p),
        None => compute_home_path(get_sys_home()?),
    }
}
pub fn set_home_from_sys() -> Result {
    set_home(Option::<&'static str>::None)
}
pub fn set_home<T: AsRef<Path>>(p: Option<T>) -> Result {
    let path = compute_home_path_optional(p)?;
    PATH.set(path);
    Ok(())
}

#[cfg(not(test))]
pub fn get_home() -> &'static Path {
    PATH.get().as_ref()
}
#[cfg(test)]
pub fn get_home() -> &'static Path {
    crate::set_once!(PATH, || { join_path(".", ".test_hyper_scripter").unwrap() });
    PATH.get().as_ref()
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
            .ok_or_else(|| Error::msg("檔案實體為空...?"))?
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
    let name = id.into_script_name().unwrap();
    let path = open_script(&name, ty, None)?; // NOTE: max + 1 的邏輯已足以確保不會產生衝突的腳本，不檢查了！
    Ok((name, path))
}

/// 若 `check_exist` 有值，則會檢查存在性
/// 需注意：要找已存在的腳本時，允許未知的腳本類型
/// 此情況下會使用 to_file_path_fallback 方法，即以類型名當作擴展名
pub fn open_script(
    name: &ScriptName,
    ty: &ScriptType,
    check_exist: Option<bool>,
) -> Result<PathBuf> {
    let script_path = if check_exist == Some(true) {
        name.to_file_path_fallback(ty)
    } else {
        name.to_file_path(ty)?
    };
    let script_path = get_home().join(script_path);

    if let Some(should_exist) = check_exist {
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

pub fn get_template_path(ty: &ScriptType) -> Result<PathBuf> {
    let dir = get_home().join(TEMPLATE);
    if !dir.exists() {
        log::info!("找不到模板資料夾，創建之");
        handle_fs_res(&[&dir], create_dir(&dir))?;
    }
    Ok(dir.join(format!("{}.hbs", ty)))
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
        let (name, p) = open_new_anonymous(&"sh".into()).unwrap();
        assert_eq!(name, ScriptName::Anonymous(6));
        assert_eq!(
            p,
            join_path("./.test_hyper_scripter/.anonymous", "6.sh").unwrap()
        );
        let p = open_script(&5.into_script_name().unwrap(), &"js".into(), None).unwrap();
        assert_eq!(
            p,
            join_path("./.test_hyper_scripter/.anonymous", "5.js").unwrap()
        );
    }
    #[test]
    fn test_open() {
        let second_name = "second".to_owned().into_script_name().unwrap();
        let not_exist = "not-exist".to_owned().into_script_name().unwrap();

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

        match open_script(&not_exist, &"sh".into(), Some(true)).unwrap_err() {
            Error::PathNotFound(name) => assert_eq!(name[0], get_home().join("not-exist.sh")),
            _ => unreachable!(),
        }

        // NOTE: 如果是要找已存在的腳本，可以允許為不存在的類型，此情況下直接將類別的名字當作擴展名
        let err = open_script(&second_name, &"no-such-type".into(), None).unwrap_err();
        assert!(matches!(err, Error::UnknownType(_)));
        let err = open_script(&second_name, &"no-such-type".into(), Some(false)).unwrap_err();
        assert!(matches!(err, Error::UnknownType(_)));
        let p = open_script(&second_name, &"no-such-type".into(), Some(true)).unwrap();
        assert_eq!(p, get_home().join("second.no-such-type"));
        // 用類別名當擴展名仍找不到，當然還是要報錯
        let err = open_script(&not_exist, &"no-such-type".into(), Some(true)).unwrap_err();
        assert!(matches!(err, Error::PathNotFound(_)));
    }
}
