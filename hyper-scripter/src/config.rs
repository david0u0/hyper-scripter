use crate::color::Color;
use crate::error::{DisplayError, DisplayResult, Error, FormatCode, Result};
use crate::path;
use crate::script_type::{ScriptType, ScriptTypeConfig};
use crate::tag::{TagGroup, TagSelector, TagSelectorGroup};
use crate::util;
use crate::util::{impl_de_by_from_str, impl_ser_by_to_string};
use fxhash::{FxHashMap as HashMap, FxHashSet as HashSet};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::SystemTime;

const CONFIG_FILE: &str = ".config.toml";
pub const CONFIG_FILE_ENV: &str = "HYPER_SCRIPTER_CONFIG";

crate::local_global_state!(config_state, Config, || { Default::default() });
crate::local_global_state!(runtime_conf_state, RuntimeConf, || { unreachable!() });

struct RuntimeConf {
    prompt_level: PromptLevel,
}

fn de_nonempty_vec<'de, D, T>(deserializer: D) -> std::result::Result<Vec<T>, D::Error>
where
    D: serde::de::Deserializer<'de>,
    T: Deserialize<'de>,
{
    let v: Vec<T> = Deserialize::deserialize(deserializer)?;
    if v.is_empty() {
        return Err(serde::de::Error::custom(
            FormatCode::NonEmptyArray.to_err(String::new()),
        ));
    }
    Ok(v)
}

pub fn config_file(home: &Path) -> PathBuf {
    match std::env::var(CONFIG_FILE_ENV) {
        Ok(p) => {
            log::debug!("使用環境變數設定檔：{}", p);
            p.into()
        }
        Err(_) => home.join(CONFIG_FILE),
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct NamedTagSelector {
    pub content: TagSelector,
    pub name: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub inactivated: bool,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
pub struct Alias {
    #[serde(deserialize_with = "de_nonempty_vec")]
    pub after: Vec<String>,
}
impl From<Vec<String>> for Alias {
    fn from(after: Vec<String>) -> Self {
        Alias { after }
    }
}
impl Alias {
    /// ```rust
    /// use hyper_scripter::config::Alias;
    ///
    /// fn get_args(alias: &Alias) -> (bool, Vec<&str>) {
    ///     let (is_shell, args) = alias.args();
    ///     (is_shell, args.collect())
    /// }
    ///
    /// let alias = Alias::from(vec!["!".to_owned()]);
    /// assert_eq!((false, vec!["!"]), get_args(&alias));
    ///
    /// let alias = Alias::from(vec!["!".to_owned(), "args".to_owned()]);
    /// assert_eq!((false, vec!["!", "args"]), get_args(&alias));
    ///
    /// let alias = Alias::from(vec!["! args".to_owned()]);
    /// assert_eq!((false, vec!["! args"]), get_args(&alias));
    ///
    /// let alias = Alias::from(vec!["!!".to_owned()]);
    /// assert_eq!((true, vec!["!"]), get_args(&alias));
    ///
    /// let alias = Alias::from(vec!["!ls".to_owned()]);
    /// assert_eq!((true, vec!["ls"]), get_args(&alias));
    ///
    /// let alias = Alias::from(vec!["!ls".to_owned(), "*".to_owned()]);
    /// assert_eq!((true, vec!["ls", "*"]), get_args(&alias));
    /// ```
    pub fn args(&self) -> (bool, impl Iterator<Item = &'_ str>) {
        let mut is_shell = false;
        let mut iter = self.after.iter().map(String::as_str);
        let mut first_args = iter.next().unwrap();
        let mut chars = first_args.chars();
        if chars.next() == Some('!') {
            if first_args.len() > 1 {
                if chars.next() != Some(' ') {
                    is_shell = true;
                    first_args = &first_args[1..];
                }
            }
        }

        return (is_shell, std::iter::once(first_args).chain(iter));
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
    type Err = DisplayError;
    fn from_str(s: &str) -> DisplayResult<Self> {
        let l = match s {
            "always" => PromptLevel::Always,
            "never" => PromptLevel::Never,
            "smart" => PromptLevel::Smart,
            "on-multi-fuzz" => PromptLevel::OnMultiFuzz,
            _ => return FormatCode::PromptLevel.to_display_res(s.to_owned()),
        };
        Ok(l)
    }
}
impl_ser_by_to_string!(PromptLevel);
impl_de_by_from_str!(PromptLevel);

#[derive(Display, PartialEq, Eq, Debug, Clone, Copy)]
pub enum Recent {
    #[display(fmt = "timeless")]
    Timeless,
    #[display(fmt = "no-neglect")]
    NoNeglect,
    #[display(fmt = "{} days", _0)]
    Days(u32),
}
impl Default for Recent {
    fn default() -> Self {
        Recent::NoNeglect
    }
}
impl FromStr for Recent {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let r = match s {
            "timeless" => Recent::Timeless,
            "no-neglect" => Recent::NoNeglect,
            _ => Recent::Days(s.parse()?),
        };
        Ok(r)
    }
}
impl serde::Serialize for Recent {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let Recent::Days(d) = self {
            serializer.serialize_u32(*d)
        } else {
            serializer.serialize_str(&self.to_string())
        }
    }
}
impl<'de> serde::Deserialize<'de> for Recent {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StringOrInt {
            String(String),
            Int(u32),
        }

        let t: StringOrInt = serde::Deserialize::deserialize(deserializer)?;
        match t {
            StringOrInt::String(s) => s.parse().map_err(serde::de::Error::custom),
            StringOrInt::Int(d) => Ok(Recent::Days(d)),
        }
    }
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone)]
pub struct Config {
    pub recent: Recent,
    pub main_tag_selector: TagSelector,
    #[serde(default)]
    pub caution_tags: TagGroup,
    prompt_level: PromptLevel,
    #[serde(deserialize_with = "de_nonempty_vec")]
    pub editor: Vec<String>,
    pub tag_selectors: Vec<NamedTagSelector>,
    pub alias: HashMap<String, Alias>,
    pub types: HashMap<ScriptType, ScriptTypeConfig>,
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
            recent: Default::default(),
            editor: vec!["vim".to_string(), "-p".to_string()],
            prompt_level: PromptLevel::Smart,
            tag_selectors: vec![
                NamedTagSelector {
                    content: "+pin,util".parse().unwrap(),
                    name: "pin".to_owned(),
                    inactivated: false,
                },
                NamedTagSelector {
                    content: "+^hide!".parse().unwrap(),
                    name: "no-hidden".to_owned(),
                    inactivated: false,
                },
                NamedTagSelector {
                    content: "+^remove!".parse().unwrap(),
                    name: "no-removed".to_owned(),
                    inactivated: false,
                },
            ],
            main_tag_selector: "+all".parse().unwrap(),
            caution_tags: "caution".parse().unwrap(),
            types: ScriptTypeConfig::default_script_types(),
            alias: [
                gen_alias("la", &["ls", "-a"]),
                gen_alias("ll", &["ls", "-l"]),
                gen_alias("l", &["ls", "--grouping", "none", "--limit", "5"]),
                gen_alias("e", &["edit"]),
                gen_alias("gc", &["rm", "--timeless", "--purge", "-s", "remove", "*"]),
                gen_alias("t", &["tags"]),
                gen_alias("p", &["run", "--previous"]),
                gen_alias("conf", &["!$HS_EDITOR $HS_HOME/.config.toml"]),
                gen_alias(
                    "pc",
                    &["=util/historian!", "--sequence", "c", "--display=all"],
                ),
                gen_alias(
                    "pr",
                    &["=util/historian!", "--sequence", "r", "--display=all"],
                ),
                gen_alias("h", &["=util/historian!", "--display=all"]),
                // Showing humble events of all scripts will be a mess
                gen_alias(
                    "hh",
                    &["=util/historian!", "*", "--display=all", "--no-humble"],
                ),
            ]
            .into_iter()
            .collect(),
            env: [
                ("NAME", "{{name}}"),
                ("HS_HOME", "{{home}}"),
                ("HS_CMD", "{{cmd}}"),
                ("HS_RUN_ID", "{{run_id}}"),
                (
                    "HS_EDITOR",
                    "{{#each editor}}{{{this}}}{{#unless @last}} {{/unless}}{{/each}}",
                ),
                (
                    "HS_TAGS",
                    "{{#each tags}}{{{this}}}{{#unless @last}} {{/unless}}{{/each}}",
                ),
                (
                    "HS_ENV_DESC",
                    "{{#each env_desc}}{{{this}}}{{#unless @last}}\n{{/unless}}{{/each}}",
                ),
                ("HS_EXE", "{{exe}}"),
                ("HS_SOURCE", "{{home}}/.hs_source"),
                ("TMP_DIR", "/tmp"),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect(),
        }
    }
}
impl Config {
    pub fn load(home: &Path) -> Result<Self> {
        let path = config_file(home);
        log::info!("載入設定檔：{:?}", path);
        match util::read_file(&path) {
            Ok(s) => {
                let meta = util::handle_fs_res(&[&path], std::fs::metadata(&path))?;
                let modified = util::handle_fs_res(&[&path], meta.modified())?;

                let mut conf: Config = toml::from_str(&s).map_err(|err| {
                    FormatCode::Config.to_err(format!("{}: {}", path.to_string_lossy(), err))
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
        config_state::set(Config::load(path::get_home())?);
        Ok(())
    }

    pub fn set_runtime_conf(prompt_level: Option<PromptLevel>) {
        let c = Config::get();
        let prompt_level = prompt_level.unwrap_or(c.prompt_level); // TODO: 測試由設定檔設定 prompt-level 的情境？
        runtime_conf_state::set(RuntimeConf { prompt_level });
    }
    pub fn get_prompt_level() -> PromptLevel {
        runtime_conf_state::get().prompt_level
    }

    pub fn get() -> &'static Config {
        config_state::get()
    }

    // XXX: extract
    pub fn gen_env(
        &self,
        info: &crate::util::TmplVal<'_>,
        strict: bool,
    ) -> Result<Vec<(String, String)>> {
        let reg = Handlebars::new();
        let mut env: Vec<(String, String)> = Vec::with_capacity(self.env.len());
        for (name, e) in self.env.iter() {
            match reg.render_template(e, info) {
                Ok(res) => env.push((name.to_owned(), res)),
                Err(err) => {
                    if strict {
                        return Err(err.into());
                    }
                }
            }
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
    pub fn get_tag_selector_group(&self, toggle: &mut HashSet<String>) -> TagSelectorGroup {
        let mut group = TagSelectorGroup::default();
        for f in self.tag_selectors.iter() {
            let inactivated = f.inactivated ^ toggle.remove(&f.name);
            if inactivated {
                log::debug!("{:?} 未啟用", f);
                continue;
            }
            group.push(f.content.clone()); // TODO: TagSelectorGroup 可以多帶點 lifetime 減少複製
        }
        group.push(self.main_tag_selector.clone());
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
            main_tag_selector: "a,^b,c".parse().unwrap(),
            ..Default::default()
        };
        let s = to_string_pretty(&c1).unwrap();
        let c2: Config = from_str(&s).unwrap();
        assert_eq!(c1, c2);
    }
}
