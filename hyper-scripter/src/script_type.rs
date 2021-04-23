use crate::error::Error;
use fxhash::FxHashMap as HashMap;
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

const SHELL_WELCOME_MSG: &str = "# [HS_HELP]: Help message goes here...

set -e
export VAR=\"${VAR:-default}\"
{{#if birthplace}}
cd ~/{{birthplace}}
{{else}}
cd {{birthplace_abs}}
{{/if}}

{{#each content}}{{{this}}}
{{/each}}";

const JS_WELCOME_MSG: &str = "// [HS_HELP]: Help message goes here...

process.chdir(require('os').homedir());
{{#if birthplace}}
process.chdir(process.env.HOME);
process.chdir('{{birthplace}}');
{{else}}
process.chdir('{{birthplace_abs}}');
{{/if}}
let spawn = require('child_process').spawnSync;
spawn('test', [], { stdio: 'inherit' });

let writeFile = require('fs').writeFileSync;
writeFile('/dev/null', 'some content');

{{#each content}}{{{this}}}
{{/each}}";

const TMUX_WELCOME_MSG: &str = "# [HS_HELP]: Help message goes here...

set -e
export VAR=\"${VAR:-default}\"
{{#if birthplace}}
cd ~/{{birthplace}}
{{else}}
cd {{birthplace_abs}}
{{/if}}

tmux new-session -s $NAME -d \"{{{content.0}}}; $SHELL\" || exit 1
tmux split-window -h \"{{{content.1}}}; $SHELL\"
{{#if content.2}}tmux split-window -v \"{{{content.2}}}; $SHELL\"
{{/if}}
tmux -2 attach-session -d";

const RB_WELCOME_MSG: &str = "# [HS_HELP]: Help message goes here...
Dir.chdir(\"#{ENV['HOME']}/{{birthplace}}\")
{{#if birthplace}}
Dir.chdir(\"#{ENV['HOME']}/{{birthplace}}\")
{{else}}
Dir.chdir(\"#{ENV['HOME']}/{{birthplace_abs}}\")
{{/if}}

{{#each content}}{{{this}}}
{{/each}} ";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct ScriptType(String);
impl From<&str> for ScriptType {
    fn from(s: &str) -> Self {
        s.parse().unwrap()
    }
}
impl From<String> for ScriptType {
    fn from(s: String) -> Self {
        // TODO: 檢查
        ScriptType(s)
    }
}
impl AsRef<str> for ScriptType {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl FromStr for ScriptType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.to_owned().into())
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
    pub template: Vec<String>,
    pub cmd: Option<String>,
    args: Vec<String>,
    env: Vec<(String, String)>,
}
fn split(s: &str) -> Vec<String> {
    s.split("\n").map(|s| s.to_owned()).collect()
}
fn default_template() -> Vec<String> {
    vec![
        "{{#each content}}{{{this}}}".to_owned(),
        "{{/each}}".to_owned(),
    ]
}

impl ScriptTypeConfig {
    // XXX: extract
    pub fn args(&self, info: &serde_json::Value) -> Result<Vec<String>, Error> {
        let reg = Handlebars::new();
        let mut args: Vec<String> = Vec::with_capacity(self.args.len());
        for c in self.args.iter() {
            let res = reg.render_template(c, &info)?;
            args.push(res);
        }
        Ok(args)
    }
    // XXX: extract
    pub fn gen_env(&self, info: &serde_json::Value) -> Result<Vec<(String, String)>, Error> {
        let reg = Handlebars::new();
        let mut env: Vec<(String, String)> = Vec::with_capacity(self.env.len());
        for (name, e) in self.env.iter() {
            let res = reg.render_template(e, &info)?;
            env.push((name.to_owned(), res));
        }
        Ok(env)
    }
    pub fn default_script_types() -> HashMap<ScriptType, ScriptTypeConfig> {
        let mut ret = HashMap::default();
        ret.insert(
            "sh".into(),
            ScriptTypeConfig {
                ext: Some("sh".to_owned()),
                color: "bright magenta".to_owned(),
                template: split(SHELL_WELCOME_MSG),
                cmd: Some("bash".to_owned()),
                args: vec!["{{path}}".to_owned()],
                env: vec![],
            },
        );
        ret.insert(
            "tmux".into(),
            ScriptTypeConfig {
                ext: Some("sh".to_owned()),
                color: "white".to_owned(),
                template: split(TMUX_WELCOME_MSG),
                cmd: Some("bash".to_owned()),
                args: vec!["{{path}}".to_owned()],
                env: vec![],
            },
        );
        ret.insert(
            "js".into(),
            ScriptTypeConfig {
                ext: Some("js".to_owned()),
                color: "bright cyan".to_owned(),
                template: split(JS_WELCOME_MSG),
                cmd: Some("node".to_owned()),
                args: vec!["{{path}}".to_owned()],
                env: vec![(
                    "NODE_PATH".to_owned(),
                    "{{{script_dir}}}/node_modules".to_owned(),
                )],
            },
        );
        ret.insert(
            "js-i".into(),
            ScriptTypeConfig {
                ext: Some("js".to_owned()),
                color: "bright cyan".to_owned(),
                template: split(JS_WELCOME_MSG),
                cmd: Some("node".to_owned()),
                args: vec!["-i".to_owned(), "-e".to_owned(), "{{{content}}}".to_owned()],
                env: vec![(
                    "NODE_PATH".to_owned(),
                    "{{{script_dir}}}/node_modules".to_owned(),
                )],
            },
        );
        ret.insert(
            "rb".into(),
            ScriptTypeConfig {
                ext: Some("rb".to_owned()),
                color: "bright red".to_owned(),
                template: split(RB_WELCOME_MSG),
                cmd: Some("ruby".to_owned()),
                args: vec!["{{path}}".to_owned()],
                env: vec![],
            },
        );

        ret.insert(
            "txt".into(),
            ScriptTypeConfig {
                ext: None,
                color: "bright black".to_owned(),
                template: default_template(),
                cmd: Some("cat".to_owned()),
                args: vec!["{{path}}".to_owned()],
                env: vec![],
            },
        );

        ret
    }
}
