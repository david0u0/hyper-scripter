use crate::error::{Error, Result};
use crate::path;
use crate::tag::{Tag, TagFilters};
use crate::util;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::PathBuf;
use std::str::FromStr;

const CONFIG_FILE: &'static str = "config.toml";

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
#[derive(Deserialize, Serialize, PartialEq, Eq, Debug)]
pub struct Config {
    #[serde(serialize_with = "ser_tag_filters")]
    #[serde(deserialize_with = "de_tag_filters")]
    pub tag_filters: TagFilters,
    pub tags: Vec<Tag>,
}
impl Default for Config {
    fn default() -> Self {
        Config {
            tag_filters: FromStr::from_str("all,-hide").unwrap(),
            tags: vec![FromStr::from_str("hide").unwrap()],
        }
    }
}
impl Config {
    pub fn load() -> Result<Self> {
        match util::read_file(&config_file()) {
            Ok(s) => toml::from_str(&s).map_err(|e| e.into()),
            Err(Error::PathNotFound(_)) => Ok(Default::default()),
            Err(e) => Err(e),
        }
    }
    pub fn store(&self) -> Result<()> {
        util::write_file(&config_file(), &toml::to_string(self)?)
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
        };
        let s = to_string(&c1).unwrap();
        let c2: Config = from_str(&s).unwrap();
        assert_eq!(c1, c2);

        c2.store().unwrap();
        Config::load().unwrap();
    }
}
