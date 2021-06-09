use crate::error::{Error, FormatCode, Result};
use crate::path;
use crate::script_type::{ScriptType, ScriptTypeConfig};
use crate::tag::{TagFilter, TagFilterGroup};
use crate::util;
use colored::Color;
use fxhash::FxHashMap as HashMap;
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;
use std::path::PathBuf;
use std::time::SystemTime;

const CONFIG_FILE: &str = ".config.toml";

#[cfg(not(test))]
lazy_static::lazy_static! {
    static ref CONFIG: Result<Config> = RawConfig::load().map(|raw_config| {
        match raw_config {
            Some((conf, time)) => Config {
                changed: false,
                raw_config: conf,
                last_modified: Some(time),
            },
            _ => RawConfig::default().into()
        }
    });
}
#[cfg(test)]
lazy_static::lazy_static! {
    static ref CONFIG: Result<Config> = Ok(RawConfig::default().into());
}

fn de_nonempty_vec<'de, D, T>(deserializer: D) -> std::result::Result<Vec<T>, D::Error>
where
    D: serde::de::Deserializer<'de>,
    T: Deserialize<'de>,
{
    let v: Vec<T> = Deserialize::deserialize(deserializer)?;
    if v.is_empty() {
        return Err(serde::de::Error::custom(Error::Format(
            FormatCode::NonEmptyArray,
            Default::default(),
        )));
    }
    Ok(v)
}

fn config_file() -> PathBuf {
    path::get_home().join(CONFIG_FILE)
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct NamedTagFilter {
    pub content: TagFilter,
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

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone, Copy)]
pub enum PromptLevel {
    Always,
    Never,
    Smart,
}
impl PromptLevel {
    pub fn always(self) -> bool {
        self == PromptLevel::Always
    }
    pub fn never(self) -> bool {
        self == PromptLevel::Never
    }
    pub fn smart(self) -> bool {
        self == PromptLevel::Smart
    }
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
pub struct RawConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recent: Option<u32>,
    pub main_tag_filter: TagFilter,
    pub prompt_level: PromptLevel,
    #[serde(deserialize_with = "de_nonempty_vec")]
    pub editor: Vec<String>,
    pub tag_filters: Vec<NamedTagFilter>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub alias: HashMap<String, Alias>,
    pub categories: HashMap<ScriptType, ScriptTypeConfig>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}
#[derive(Debug, Clone, Deref)]
pub struct Config {
    changed: bool,
    last_modified: Option<SystemTime>,
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
            editor: vec!["vim".to_string()],
            prompt_level: PromptLevel::Smart,
            tag_filters: vec![
                NamedTagFilter {
                    content: "+pin".parse().unwrap(),
                    name: "pin".to_owned(),
                },
                NamedTagFilter {
                    content: "+m/all,^hide".parse().unwrap(),
                    name: "no-hidden".to_owned(),
                },
                NamedTagFilter {
                    content: "+m/all,^removed".parse().unwrap(),
                    name: "no-removed".to_owned(),
                },
            ],
            main_tag_filter: "+all".parse().unwrap(),
            categories: ScriptTypeConfig::default_script_types(),
            alias: vec![
                // FIXME: 一旦陣列實作了 intoiterator 就用陣列
                gen_alias("la", &["ls", "-a"]),
                gen_alias("ll", &["ls", "-l"]),
                gen_alias("l", &["ls", "--grouping", "none"]),
                gen_alias("e", &["edit"]),
                gen_alias("gc", &["rm", "--purge", "-f", "removed", "*"]),
                gen_alias("tree", &["ls", "--grouping", "tree"]),
                gen_alias("t", &["tags"]),
            ]
            .into_iter()
            .collect(),
            recent: Some(999999), // NOTE: 顯示兩千多年份的資料！
            // FIXME: 一旦陣列實作了 intoiterator 就用陣列
            env: vec![
                ("NAME", "{{name}}"),
                ("HS_HOME", "{{hs_home}}"),
                ("HS_CMD", "{{hs_cmd}}"),
                (
                    "HS_TAGS",
                    "{{#each hs_tags}}{{{this}}}{{#unless @last}} {{/unless}}{{/each}}",
                ),
                ("HS_EXE", "{{hs_exe}}"),
                ("HS_SOURCE", "{{hs_home}}/.hs_source"),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect(),
        }
    }
}
impl RawConfig {
    pub fn load() -> Result<Option<(Self, SystemTime)>> {
        let path = config_file();
        log::info!("載入設定檔：{:?}", path);
        match util::read_file(&path) {
            Ok(s) => {
                let meta = util::handle_fs_res(&[&path], std::fs::metadata(&path))?;
                let modified = util::handle_fs_res(&[&path], meta.modified())?;

                let conf = toml::from_str(&s).map_err(|err| {
                    Error::Format(
                        FormatCode::Config,
                        format!("{}: {}", path.to_string_lossy(), err),
                    )
                })?;
                Ok(Some((conf, modified)))
            }
            Err(Error::PathNotFound(_)) => {
                log::debug!("找不到設定檔");
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }
    // XXX: extract
    pub fn gen_env(&self, info: &serde_json::Value) -> Result<Vec<(String, String)>> {
        let reg = Handlebars::new();
        let mut env: Vec<(String, String)> = Vec::with_capacity(self.env.len());
        for (name, e) in self.env.iter() {
            let res = reg.render_template(e, &info)?;
            env.push((name.to_owned(), res));
        }
        Ok(env)
    }
}
impl DerefMut for Config {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.changed = true;
        &mut self.raw_config
    }
}
impl From<RawConfig> for Config {
    fn from(c: RawConfig) -> Self {
        Config {
            changed: true,
            last_modified: None,
            raw_config: c,
        }
    }
}
impl Config {
    pub fn store(&self) -> Result<()> {
        let path = config_file();
        log::info!("寫入設定檔至 {:?}…", path);
        if !self.changed {
            log::info!("設定檔未改變，不寫入");
            return Ok(());
        }
        match util::handle_fs_res(&[&path], std::fs::metadata(&path)) {
            Ok(meta) => {
                let modified = util::handle_fs_res(&[&path], meta.modified())?;
                if self.last_modified.map_or(false, |time| time < modified) {
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
            .ok_or_else(|| Error::UnknownType(ty.to_string()))
    }
    pub fn get_tag_filter_group(&self) -> TagFilterGroup {
        let mut group = TagFilterGroup::default();
        for f in self.tag_filters.iter() {
            group.push(f.content.clone());
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
