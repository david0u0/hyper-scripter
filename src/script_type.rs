use crate::error::Error;
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

const SHELL_WELCOME_MSG: &str = "# Hello, scripter!
# Here are some useful commands to begin with:

export VAR=\"${VAR:-default}\"
cd {{birthplace}}

{{content}}
";

const JS_WELCOME_MSG: &str = "// Hello, scripter!
// Here are some information you may be intrested in:

process.chdir(\"{{birthplace}}\");

{{content}}
";

const TMUX_WELCOME_MSG: &str = "cd {{birthplace}}
tmux new-session -s {{name}} -d ' '
tmux split-window -v ' '
tmux -2 attach-session -d

{{content}}
";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct ScriptType(String);
impl From<&str> for ScriptType {
    fn from(s: &str) -> Self {
        Self::from_str(s).unwrap()
    }
}
impl FromStr for ScriptType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // TODO: 檢查
        Ok(ScriptType(s.to_owned()))
    }
}
impl std::fmt::Display for ScriptType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl Default for ScriptType {
    fn default() -> Self {
        "sh".into()
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScriptTypeConfig {
    pub ext: Option<String>,
    pub color: String,
    pub template: String,
    pub cmd: Option<String>,
    args: Vec<String>,
}

impl ScriptTypeConfig {
    pub fn args(&self, info: &serde_json::Value) -> Result<Vec<String>, Error> {
        let reg = Handlebars::new();
        let mut args: Vec<String> = Vec::with_capacity(self.args.len());
        for c in self.args.iter() {
            let res = reg.render_template(c, &info)?;
            args.push(res);
        }
        Ok(args)
    }
    pub fn default_script_types() -> HashMap<ScriptType, ScriptTypeConfig> {
        let mut ret = HashMap::default();
        ret.insert(
            "sh".into(),
            ScriptTypeConfig {
                ext: Some("sh".to_owned()),
                color: "bright magenta".to_owned(),
                template: SHELL_WELCOME_MSG.to_owned(),
                cmd: Some("bash".to_owned()),
                args: vec!["{{path}}".to_owned()],
            },
        );
        ret.insert(
            "tmux".into(),
            ScriptTypeConfig {
                ext: Some("sh".to_owned()),
                color: "white".to_owned(),
                template: TMUX_WELCOME_MSG.to_owned(),
                cmd: Some("sh".to_owned()),
                args: vec!["{{path}}".to_owned()],
            },
        );
        ret.insert(
            "js".into(),
            ScriptTypeConfig {
                ext: Some("js".to_owned()),
                color: "bright cyan".to_owned(),
                template: JS_WELCOME_MSG.to_owned(),
                cmd: Some("node".to_owned()),
                args: vec!["{{path}}".to_owned()],
            },
        );
        ret.insert(
            "js-i".into(),
            ScriptTypeConfig {
                ext: Some("js".to_owned()),
                color: "bright cyan".to_owned(),
                template: JS_WELCOME_MSG.to_owned(),
                cmd: Some("node".to_owned()),
                args: vec!["-i".to_owned(), "-e".to_owned(), "{{{content}}}".to_owned()],
            },
        );
        ret.insert(
            "rb".into(),
            ScriptTypeConfig {
                ext: Some("rb".to_owned()),
                color: "bright red".to_owned(),
                template: "".to_owned(),
                cmd: Some("ruby".to_owned()),
                args: vec!["{{path}}".to_owned()],
            },
        );

        ret.insert(
            "md".into(),
            ScriptTypeConfig {
                ext: Some("md".to_owned()),
                color: "bright black".to_owned(),
                template: "".to_owned(),
                cmd: None,
                args: vec![],
            },
        );

        ret
    }
}