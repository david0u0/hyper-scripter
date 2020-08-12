use crate::error::{Error, Result};
use crate::path;
use crate::script_type::{ScriptType, ScriptTypeConfig};
use crate::tag::{Tag, TagFilters};
use crate::util;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

const CONFIG_FILE: &'static str = "config.toml";
lazy_static::lazy_static! {
    static ref CONFIG: Config = Config::load().unwrap();
}

fn config_file() -> PathBuf {
    path::get_path().join(CONFIG_FILE)
}

fn ser_tag_filters<S>(x: &TagFilters, s: S) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&x.to_string())
}
fn de_tag_filters<'de, D>(deserializer: D) -> std::result::Result<TagFilters, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    let filters = TagFilters::from_str(s).unwrap();
    Ok(filters)
}
#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
pub struct Config {
    #[serde(serialize_with = "ser_tag_filters")]
    #[serde(deserialize_with = "de_tag_filters")]
    pub tag_filters: TagFilters,
    pub tags: Vec<Tag>,
    pub categories: HashMap<ScriptType, ScriptTypeConfig>,
}
impl Default for Config {
    fn default() -> Self {
        Config {
            tag_filters: FromStr::from_str("all,-hide").unwrap(),
            tags: vec![FromStr::from_str("hide").unwrap()],
            categories: ScriptTypeConfig::default_script_types(),
        }
    }
}
impl Config {
    fn load() -> Result<Self> {
        let path = config_file();
        log::info!("載入設定檔：{:?}", path);
        match util::read_file(&path) {
            Ok(s) => toml::from_str(&s).map_err(|e| e.into()),
            Err(Error::PathNotFound(_)) => {
                log::debug!("找不到設定檔，使用預設值");
                Ok(Default::default())
            }
            Err(e) => Err(e),
        }
    }
    pub fn store(&self) -> Result<()> {
        util::write_file(&config_file(), &toml::to_string(self)?)
    }
    pub fn get() -> &'static Config {
        &CONFIG
    }
    pub fn get_script_conf(&self, ty: &ScriptType) -> Result<&ScriptTypeConfig> {
        self.categories
            .get(ty)
            .ok_or(Error::UnknownCategory(ty.to_string()))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use toml::{from_str, to_string};
    #[test]
    fn test_config_serde() {
        path::set_path_from_sys().unwrap();
        let c1 = Config {
            tags: vec![FromStr::from_str("測試標籤").unwrap()],
            tag_filters: FromStr::from_str("a,-b,c").unwrap(),
            ..Default::default()
        };
        let s = to_string(&c1).unwrap();
        println!("{}", s);
        let c2: Config = from_str(&s).unwrap();
        assert_eq!(c1, c2);

        c2.store().unwrap();
        Config::load().unwrap();
    }
}
