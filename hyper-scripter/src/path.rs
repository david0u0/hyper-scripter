use crate::error::{Contextable, Error, Result};
use crate::script::IntoScriptName;
use crate::script::{ScriptName, ANONYMOUS};
use crate::script_type::{AsScriptFullTypeRef, ScriptType};
use crate::state::State;
use crate::util::{handle_fs_res, read_file};
use fxhash::FxHashSet as HashSet;
use std::fs::{create_dir, create_dir_all, read_dir};
use std::path::{Component, Path, PathBuf};

pub const HS_REDIRECT: &str = ".hs_redirect";
pub const HS_PRE_RUN: &str = ".hs_prerun";
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

fn join_here_abs<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    let path = path.as_ref();
    if path.is_absolute() {
        return Ok(path.to_owned());
    }
    // NOTE: 用 $PWD 可以取到 symlink 還沒解開前的路徑
    // 若用 std::env::current_dir，該路徑已為真實路徑
    let here = std::env::var("PWD")?;
    Ok(AsRef::<Path>::as_ref(&here).join(path))
}

pub fn normalize_path<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    let path = join_here_abs(path)?;
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    Ok(ret)
}

fn compute_home_path<T: AsRef<Path>>(p: T, create_on_missing: bool) -> Result<PathBuf> {
    let path = join_here_abs(p)?;
    log::debug!("計算路徑：{:?}", path);
    if !path.exists() {
        if create_on_missing {
            log::info!("路徑 {:?} 不存在，嘗試創建之", path);
            handle_fs_res(&[&path], create_dir(&path))?;
        } else {
            return Err(Error::PathNotFound(vec![path]));
        }
    } else {
        let redirect = path.join(HS_REDIRECT);
        if redirect.is_file() {
            let redirect = read_file(&redirect)?;
            let redirect = path.join(redirect.trim());
            log::info!("重導向至 {:?}", redirect);
            return compute_home_path(redirect, create_on_missing);
        }
    }
    Ok(path)
}
pub fn compute_home_path_optional<T: AsRef<Path>>(
    p: Option<T>,
    create_on_missing: bool,
) -> Result<PathBuf> {
    match p {
        Some(p) => compute_home_path(p, create_on_missing),
        None => compute_home_path(get_sys_home()?, create_on_missing),
    }
}
pub fn set_home<T: AsRef<Path>>(p: Option<T>, create_on_missing: bool) -> Result {
    let path = compute_home_path_optional(p, create_on_missing)?;
    PATH.set(path);
    Ok(())
}

#[cfg(not(test))]
pub fn get_home() -> &'static Path {
    PATH.get().as_ref()
}
#[cfg(test)]
pub fn get_test_home() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.join(".test_hyper_scripter")
}

#[cfg(test)]
pub fn get_home() -> &'static Path {
    crate::set_once!(PATH, || { get_test_home() });
    PATH.get().as_ref()
}

fn get_anonymous_ids() -> Result<Vec<u32>> {
    // TODO: iterator
    let dir = get_home().join(ANONYMOUS);
    if !dir.exists() {
        log::info!("找不到匿名腳本資料夾，創建之");
        handle_fs_res(&[&dir], create_dir(&dir))?;
    }

    let mut ids = vec![];
    let re = regex::Regex::new(r"\..+$").unwrap();
    for entry in handle_fs_res(&[&dir], read_dir(&dir))? {
        let name = entry?.file_name();
        let name = name
            .to_str()
            .ok_or_else(|| Error::msg("檔案實體為空...?"))?;
        let name = re.replace(name, "");
        match name.parse::<u32>() {
            Ok(id) => ids.push(id),
            _ => log::warn!("匿名腳本名無法轉為整數：{}", name),
        }
    }

    Ok(ids)
}
pub fn new_anonymous_name() -> Result<ScriptName> {
    let ids: HashSet<_> = get_anonymous_ids()
        .context("無法取得匿名腳本編號")?
        .into_iter()
        .collect();
    let mut i = 1;
    loop {
        if !ids.contains(&i) {
            return i.into_script_name();
        }
        i += 1;
    }
}
pub fn open_new_anonymous(ty: &ScriptType) -> Result<(ScriptName, PathBuf)> {
    let name = new_anonymous_name()?;
    let path = open_script(&name, ty, None)?; // NOTE: new_anonymous_name 的邏輯已足以確保不會產生衝突的檔案，不檢查了！
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

pub fn get_template_path<T: AsScriptFullTypeRef>(ty: &T) -> Result<PathBuf> {
    let p = get_home()
        .join(TEMPLATE)
        .join(format!("{}.hbs", ty.display()));
    if let Some(dir) = p.parent() {
        if !dir.exists() {
            log::info!("找不到模板資料夾，創建之");
            handle_fs_res(&[&dir], create_dir_all(&dir))?;
        }
    }
    Ok(p)
}
pub fn get_sub_types(ty: &ScriptType) -> Result<Vec<ScriptType>> {
    let dir = get_home().join(TEMPLATE).join(ty.as_ref());
    if !dir.exists() {
        log::info!("找不到子類別資料夾，直接回傳");
        return Ok(vec![]);
    }

    let mut subs = vec![];
    let re = regex::Regex::new(r"\.hbs$").unwrap();
    for entry in handle_fs_res(&[&dir], read_dir(&dir))? {
        let name = entry?.file_name();
        let name = name
            .to_str()
            .ok_or_else(|| Error::msg("檔案實體為空...?"))?;
        let name = re.replace(&name, "");
        subs.push(name.parse()?);
    }
    Ok(subs)
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
        assert_eq!(name, ScriptName::Anonymous(3));
        assert_eq!(p, get_test_home().join(".anonymous/3.sh"));
        let p = open_script(&5.into_script_name().unwrap(), &"js".into(), None).unwrap();
        assert_eq!(p, get_test_home().join(".anonymous/5.js"));
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
        assert_eq!(p, get_test_home().join(".anonymous/1.sh"));

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
