use crate::error::{Error, FormatCode, Result};
use crate::path;
use crate::script_type::{ScriptType, ScriptTypeConfig};
use crate::state::State;
use crate::tag::{TagFilter, TagFilterGroup};
use crate::util;
use crate::{impl_de_by_from_str, impl_ser_by_to_string};
use colored::Color;
use fxhash::{FxHashMap as HashMap, FxHashSet as HashSet};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::SystemTime;

const CONFIG_FILE: &str = ".config.toml";

static CONFIG: State<Config> = State::new();
static PROMPT_LEVEL: State<PromptLevel> = State::new();

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

fn config_file(home: &Path) -> PathBuf {
    home.join(CONFIG_FILE)
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct NamedTagFilter {
    pub content: TagFilter,
    pub name: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub inactivated: bool,
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

#[derive(Display, PartialEq, Eq, Debug, Clone, Copy)]
pub enum PromptLevel {
    #[display(fmt = "always")]
    Always,
    #[display(fmt = "never")]
    Never,
    #[display(fmt = "smart")]
    Smart,
    #[display(fmt = "on_multi_fuzz")]
    OnMultiFuzz,
}
impl FromStr for PromptLevel {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        let l = match s {
            "always" => PromptLevel::Always,
            "never" => PromptLevel::Never,
            "smart" => PromptLevel::Smart,
            "on-multi-fuzz" => PromptLevel::OnMultiFuzz,
            _ => return Err(Error::Format(FormatCode::PromptLevel, s.to_owned())),
        };
        Ok(l)
    }
}
impl_ser_by_to_string!(PromptLevel);
impl_de_by_from_str!(PromptLevel);

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recent: Option<u32>,
    pub main_tag_filter: TagFilter,
    prompt_level: PromptLevel,
    #[serde(deserialize_with = "de_nonempty_vec")]
    pub editor: Vec<String>,
    pub tag_filters: Vec<NamedTagFilter>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub alias: HashMap<String, Alias>,
    pub types: HashMap<ScriptType, ScriptTypeConfig>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
    #[serde(skip)]
    last_modified: Option<SystemTime>,
}
impl Default for Config {
    fn default() -> Self {
        fn gen_alias(from: &str, after: &[&str]) -> (String, Alias) {
            (
                from.to_owned(),
                Alias {
                    after: after.iter().map(|s| s.to_string()).collect(),
                },
            )
        }
        Config {
            last_modified: None,
            recent: Some(999999), // NOTE: 顯示兩千多年份的資料！
            editor: vec!["vim".to_string()],
            prompt_level: PromptLevel::Smart,
            tag_filters: vec![
                NamedTagFilter {
                    content: "+pin".parse().unwrap(),
                    name: "pin".to_owned(),
                    inactivated: false,
                },
                NamedTagFilter {
                    content: "+all,^hide!".parse().unwrap(),
                    name: "no-hidden".to_owned(),
                    inactivated: false,
                },
                NamedTagFilter {
                    content: "+all,^removed!".parse().unwrap(),
                    name: "no-removed".to_owned(),
                    inactivated: false,
                },
            ],
            main_tag_filter: "+all".parse().unwrap(),
            types: ScriptTypeConfig::default_script_types(),
            alias: std::array::IntoIter::new([
                gen_alias("la", &["ls", "-a"]),
                gen_alias("ll", &["ls", "-l"]),
                gen_alias("l", &["ls", "--grouping", "none"]),
                gen_alias("e", &["edit"]),
                gen_alias("gc", &["rm", "--timeless", "--purge", "-f", "removed", "*"]),
                gen_alias("tree", &["ls", "--grouping", "tree"]),
                gen_alias("t", &["tags"]),
                gen_alias("p", &["run", "--previous-args"]),
                gen_alias("pc", &["=util/historian!", "c", "--"]),
                gen_alias("h", &["=util/historian!"]),
            ])
            .collect(),
            env: std::array::IntoIter::new([
                ("NAME", "{{name}}"),
                ("HS_HOME", "{{home}}"),
                ("HS_CMD", "{{cmd}}"),
                ("HS_RUN_ID", "{{run_id}}"),
                (
                    "HS_TAGS",
                    "{{#each tags}}{{{this}}}{{#unless @last}} {{/unless}}{{/each}}",
                ),
                (
                    "HS_ENV_HELP",
                    "{{#each env_help}}{{{this}}}{{#unless @last}}\n{{/unless}}{{/each}}",
                ),
                ("HS_EXE", "{{exe}}"),
                ("HS_SOURCE", "{{home}}/.hs_source"),
            ])
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect(),
        }
    }
}
impl Config {
    pub fn load(p: &Path) -> Result<Self> {
        let path = config_file(p);
        log::info!("載入設定檔：{:?}", path);
        match util::read_file(&path) {
            Ok(s) => {
                let meta = util::handle_fs_res(&[&path], std::fs::metadata(&path))?;
                let modified = util::handle_fs_res(&[&path], meta.modified())?;

                let mut conf: Config = toml::from_str(&s).map_err(|err| {
                    Error::Format(
                        FormatCode::Config,
                        format!("{}: {}", path.to_string_lossy(), err),
                    )
                })?;
                conf.last_modified = Some(modified);
                Ok(conf)
            }
            Err(Error::PathNotFound(_)) => {
                log::debug!("找不到設定檔");
                Ok(Default::default())
            }
            Err(e) => Err(e),
        }
    }

    pub fn store(&self) -> Result {
        let path = config_file(path::get_home());
        log::info!("寫入設定檔至 {:?}…", path);
        match util::handle_fs_res(&[&path], std::fs::metadata(&path)) {
            Ok(meta) => {
                let modified = util::handle_fs_res(&[&path], meta.modified())?;
                // NOTE: 若設定檔是憑空造出來的，但要存入時卻發現已有檔案，同樣不做存入
                if self.last_modified.map_or(true, |time| time < modified) {
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

    pub fn is_from_dafault(&self) -> bool {
        self.last_modified.is_none()
    }

    pub fn init() -> Result {
        CONFIG.set(Config::load(path::get_home())?);
        Ok(())
    }

    pub fn set_prompt_level(l: Option<PromptLevel>) {
        let c = Config::get();
        let l = l.unwrap_or(c.prompt_level); // TODO: 測試由設定檔設定 prompt-level 的情境？
        PROMPT_LEVEL.set(l);
    }
    pub fn get_prompt_level() -> PromptLevel {
        *PROMPT_LEVEL.get()
    }

    #[cfg(not(test))]
    pub fn get() -> &'static Config {
        CONFIG.get()
    }
    #[cfg(test)]
    pub fn get() -> &'static Config {
        crate::set_once!(CONFIG, || { Config::default().into() });
        CONFIG.get()
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
    pub fn get_color(&self, ty: &ScriptType) -> Result<Color> {
        let c = self.get_script_conf(ty)?.color.as_str();
        Ok(Color::from(c))
    }
    pub fn get_script_conf(&self, ty: &ScriptType) -> Result<&ScriptTypeConfig> {
        self.types
            .get(ty)
            .ok_or_else(|| Error::UnknownType(ty.to_string()))
    }
    pub fn get_tag_filter_group(&self, toggle: &mut HashSet<String>) -> TagFilterGroup {
        let mut group = TagFilterGroup::default();
        for f in self.tag_filters.iter() {
            let inactivated = f.inactivated ^ toggle.remove(&f.name);
            if inactivated {
                log::debug!("{:?} 未啟用", f);
                continue;
            }
            group.push(f.content.clone()); // TODO: TagFilterGroup 可以多帶點 lifetime 減少複製
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
        let c1 = Config {
            main_tag_filter: "a,^b,c".parse().unwrap(),
            ..Default::default()
        };
        let s = to_string_pretty(&c1).unwrap();
        let c2: Config = from_str(&s).unwrap();
        assert_eq!(c1, c2);
    }
}
