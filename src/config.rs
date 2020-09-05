use crate::error::{Error, Result};
use crate::path;
use crate::script_type::{ScriptType, ScriptTypeConfig};
use crate::tag::{TagControlFlow, TagFilter, TagFilterGroup};
use crate::util;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

const CONFIG_FILE: &'static str = "config.toml";
lazy_static::lazy_static! {
    static ref CONFIG: Result<Config> = Config::load();
}

fn config_file() -> PathBuf {
    path::get_path().join(CONFIG_FILE)
}

fn is_false(t: &bool) -> bool {
    !t
}
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct NamedTagFilter {
    pub filter: TagControlFlow,
    #[serde(default, skip_serializing_if = "is_false")]
    pub must: bool,
    pub name: String,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
pub struct Config {
    #[serde(skip_serializing, default = "Utc::now")]
    pub open_time: DateTime<Utc>,
    pub tag_filters: Vec<NamedTagFilter>,
    pub main_tag_filter: TagFilter,
    pub categories: HashMap<ScriptType, ScriptTypeConfig>,
}
impl Default for Config {
    fn default() -> Self {
        Config {
            tag_filters: vec![
                NamedTagFilter {
                    filter: FromStr::from_str("pin").unwrap(),
                    must: false,
                    name: "pin".to_owned(),
                },
                NamedTagFilter {
                    filter: FromStr::from_str("all,^deleted").unwrap(),
                    must: true,
                    name: "no-deleted".to_owned(),
                },
            ],
            main_tag_filter: FromStr::from_str("all,^hide").unwrap(),
            categories: ScriptTypeConfig::default_script_types(),
            open_time: Utc::now(),
        }
    }
}
impl Config {
    fn load() -> Result<Self> {
        let path = config_file();
        log::info!("載入設定檔：{:?}", path);
        match util::read_file(&path) {
            Ok(s) => toml::from_str(&s).map_err(|_| Error::Format(s)),
            Err(Error::PathNotFound(_)) => {
                log::debug!("找不到設定檔，使用預設值");
                Ok(Default::default())
            }
            Err(e) => Err(e),
        }
    }
    pub fn store(&self) -> Result<()> {
        log::info!("寫入設定檔…");
        let path = config_file();
        match util::handle_fs_res(&[&path], std::fs::metadata(&path)) {
            Ok(meta) => {
                let modified = util::handle_fs_res(&[&path], meta.modified())?;
                let modified = modified.duration_since(std::time::UNIX_EPOCH)?.as_secs();
                if self.open_time.timestamp() < modified as i64 {
                    log::info!("設定檔已被修改，不寫入");
                    return Ok(());
                }
            }
            Err(Error::PathNotFound(_)) => {
                log::debug!("設定檔不存在，寫就對了");
            }
            Err(err) => return Err(err),
        }
        util::write_file(&path, &toml::to_string_pretty(self)?)
    }
    pub fn get() -> Result<&'static Config> {
        match &*CONFIG {
            Ok(c) => Ok(c),
            Err(e) => Err(e.clone()),
        }
    }
    pub fn get_script_conf(&self, ty: &ScriptType) -> Result<&ScriptTypeConfig> {
        self.categories
            .get(ty)
            .ok_or(Error::UnknownCategory(ty.to_string()))
    }
    pub fn get_tag_filter_group(&self) -> TagFilterGroup {
        let mut group = TagFilterGroup::default();
        for f in self.tag_filters.iter() {
            group.push(TagFilter {
                must: f.must,
                filter: f.filter.clone(),
            });
        }
        group.push(self.main_tag_filter.clone());
        group
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use toml::{from_str, to_string_pretty};
    #[test]
    fn test_config_serde() {
        path::set_path_from_sys().unwrap();
        let c1 = Config {
            main_tag_filter: FromStr::from_str("a,^b,c").unwrap(),
            ..Default::default()
        };
        let s = to_string_pretty(&c1).unwrap();
        println!("{}", s);
        let mut c2: Config = from_str(&s).unwrap();
        c2.open_time = c1.open_time;
        assert_eq!(c1, c2);

        c2.store().unwrap();
        Config::load().unwrap();
    }
}
