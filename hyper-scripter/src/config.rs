use crate::error::{Error, Result};
use crate::path;
use crate::script_type::{ScriptType, ScriptTypeConfig};
use crate::tag::{TagControlFlow, TagFilter, TagFilterGroup};
use crate::util;
use chrono::{DateTime, Utc};
use colored::Color;
use fxhash::FxHashMap as HashMap;
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;
use std::path::PathBuf;

const CONFIG_FILE: &'static str = ".config.toml";
lazy_static::lazy_static! {
    static ref CONFIG: Result<Config> = RawConfig::load().map(|c| {
        Config {
            changed: false,
            raw_config: c,
            open_time: Utc::now(),
        }
    });
}

fn config_file() -> PathBuf {
    path::get_home().join(CONFIG_FILE)
}

fn is_false(t: &bool) -> bool {
    !t
}
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct NamedTagFilter {
    pub filter: TagControlFlow,
    #[serde(default, skip_serializing_if = "is_false")]
    pub obligation: bool,
    pub name: String,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
pub struct Alias {
    pub after: Vec<String>,
}
impl From<Vec<String>> for Alias {
    fn from(after: Vec<String>) -> Self {
        Alias { after }
    }
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
pub struct RawConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recent: Option<u32>,
    pub main_tag_filter: TagFilter,
    pub tag_filters: Vec<NamedTagFilter>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub alias: HashMap<String, Alias>,
    pub categories: HashMap<ScriptType, ScriptTypeConfig>,
}
#[derive(Debug, Clone, Deref)]
pub struct Config {
    changed: bool,
    open_time: DateTime<Utc>,
    #[deref]
    raw_config: RawConfig,
}
impl Default for RawConfig {
    fn default() -> Self {
        fn gen_alias(from: &str, after: &[&str]) -> (String, Alias) {
            (
                from.to_owned(),
                Alias {
                    after: after.iter().map(|s| s.to_string()).collect(),
                },
            )
        }
        RawConfig {
            tag_filters: vec![
                NamedTagFilter {
                    filter: "+pin".parse().unwrap(),
                    obligation: false,
                    name: "pin".to_owned(),
                },
                NamedTagFilter {
                    filter: "+all,^hide".parse().unwrap(),
                    obligation: true,
                    name: "no-hidden".to_owned(),
                },
                NamedTagFilter {
                    filter: "+all,^removed".parse().unwrap(),
                    obligation: true,
                    name: "no-removed".to_owned(),
                },
            ],
            main_tag_filter: "+all".parse().unwrap(),
            categories: ScriptTypeConfig::default_script_types(),
            alias: vec![
                // FIXME: 一旦陣列實作了 intoiterator 就用陣列
                gen_alias("la", &["ls", "-a"]),
                gen_alias("ll", &["ls", "-l"]),
                gen_alias("l", &["ls"]),
                gen_alias("e", &["edit"]),
                gen_alias("gc", &["rm", "--purge", "-f", "removed", "*"]),
                gen_alias("tree", &["ls", "--grouping", "tree"]),
                gen_alias("t", &["tags"]),
            ]
            .into_iter()
            .collect(),
            recent: Some(999999), // NOTE: 顯示兩千多年份的資料！
        }
    }
}
impl RawConfig {
    #[cfg(test)]
    fn load() -> Result<Self> {
        Ok(Self::default())
    }
    #[cfg(not(test))]
    fn load() -> Result<Self> {
        use crate::error::FormatCode;

        let path = config_file();
        log::info!("載入設定檔：{:?}", path);
        match util::read_file(&path) {
            Ok(s) => toml::from_str(&s)
                .map_err(|_| Error::Format(FormatCode::Config, path.to_string_lossy().into())),
            Err(Error::PathNotFound(_)) => {
                log::debug!("找不到設定檔，使用預設值");
                Ok(Default::default())
            }
            Err(e) => Err(e),
        }
    }
}
impl DerefMut for Config {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.changed = true;
        &mut self.raw_config
    }
}
impl Config {
    pub fn store(&self) -> Result<()> {
        log::info!("寫入設定檔…");
        let path = config_file();
        if !self.changed && path.exists() {
            log::info!("設定檔未改變，不寫入");
            return Ok(());
        }
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
        util::write_file(&path, &toml::to_string_pretty(&**self)?)
    }
    pub fn get() -> Result<&'static Config> {
        match &*CONFIG {
            Ok(c) => Ok(c),
            Err(e) => Err(e.clone()),
        }
    }
    pub fn get_color(&self, ty: &ScriptType) -> Result<Color> {
        let c = self.get_script_conf(ty)?.color.as_str();
        Ok(Color::from(c))
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
                obligation: f.obligation,
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
        path::set_home_from_sys().unwrap();
        let c1 = RawConfig {
            main_tag_filter: "a,^b,c".parse().unwrap(),
            ..Default::default()
        };
        let s = to_string_pretty(&c1).unwrap();
        let c2: RawConfig = from_str(&s).unwrap();
        assert_eq!(c1, c2);
    }
}
