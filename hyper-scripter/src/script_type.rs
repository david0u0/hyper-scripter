use crate::error::Error;
use fxhash::FxHashMap as HashMap;
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

const DEFAULT_WELCOME_MSG: &str = "{{#each content}}{{{this}}}
{{/each}}";

const SHELL_WELCOME_MSG: &str = "# [HS_HELP]: Help message goes here...

set -e
export VAR=\"${VAR:-default}\"
{{#if birthplace_in_home}}
cd ~/{{birthplace}}
{{else}}
cd {{birthplace_abs}}
{{/if}}
{{#each content}}{{{this}}}
{{/each}}";

const JS_WELCOME_MSG: &str = "// [HS_HELP]: Help message goes here...

process.chdir(require('os').homedir());
{{#if birthplace_in_home}}
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
NAME=${NAME/./_}
tmux has-session -t $NAME
if [ $? = 0 ]; then
    echo attach to existing session
    tmux -2 attach-session -t $NAME
    exit
fi

set -e
export VAR=\"${VAR:-default}\"
{{#if birthplace_in_home}}
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
{{#if birthplace_in_home}}
Dir.chdir(\"#{ENV['HOME']}/{{birthplace}}\")
{{else}}
Dir.chdir(\"{{birthplace_abs}}\")
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
    pub cmd: Option<String>,
    args: Vec<String>,
    env: Vec<(String, String)>,
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
        for (ty, conf) in iter_default_configs() {
            ret.insert(ty, conf);
        }
        ret
    }
}

macro_rules! create_default_types {
    ($(( $name:literal, $tmpl:ident, $conf:expr )),*) => {
        pub fn get_default_template(ty: &ScriptType) -> &'static str {
            match ty.0.as_ref() {
                $($name => $tmpl,)*
                _ => DEFAULT_WELCOME_MSG
            }
        }
        pub fn iter_default_templates() -> impl ExactSizeIterator<Item = (ScriptType, &'static str)> {
            let arr = [$( (ScriptType($name.to_owned()), $tmpl), )*];
            std::array::IntoIter::new(arr)
        }
        fn iter_default_configs() -> impl ExactSizeIterator<Item = (ScriptType, ScriptTypeConfig)> {
            let arr = [$( (ScriptType($name.to_owned()), $conf), )*];
            std::array::IntoIter::new(arr)
        }
    };
}

create_default_types! {
    ("sh", SHELL_WELCOME_MSG, ScriptTypeConfig {
        ext: Some("sh".to_owned()),
        color: "bright magenta".to_owned(),
        cmd: Some("bash".to_owned()),
        args: vec!["{{path}}".to_owned()],
        env: vec![],
    }),
    ("tmux", TMUX_WELCOME_MSG, ScriptTypeConfig {
        ext: Some("sh".to_owned()),
        color: "white".to_owned(),
        cmd: Some("bash".to_owned()),
        args: vec!["{{path}}".to_owned()],
        env: vec![],
    }),
    ("js", JS_WELCOME_MSG, ScriptTypeConfig {
        ext: Some("js".to_owned()),
        color: "bright cyan".to_owned(),
        cmd: Some("node".to_owned()),
        args: vec!["{{path}}".to_owned()],
        env: vec![(
            "NODE_PATH".to_owned(),
            "{{{hs_home}}}/node_modules".to_owned(),
        )],
    }),
    ("js-i", JS_WELCOME_MSG, ScriptTypeConfig {
        ext: Some("js".to_owned()),
        color: "bright cyan".to_owned(),
        cmd: Some("node".to_owned()),
        args: vec!["-i".to_owned(), "-e".to_owned(), "{{{content}}}".to_owned()],
        env: vec![(
            "NODE_PATH".to_owned(),
            "{{{hs_home}}}/node_modules".to_owned(),
        )],
    }),
    ("rb", RB_WELCOME_MSG, ScriptTypeConfig {
        ext: Some("rb".to_owned()),
        color: "bright red".to_owned(),
        cmd: Some("ruby".to_owned()),
        args: vec!["{{path}}".to_owned()],
        env: vec![],
    }),
    ("txt", DEFAULT_WELCOME_MSG, ScriptTypeConfig {
        ext: None,
        color: "bright black".to_owned(),
        cmd: Some("cat".to_owned()),
        args: vec!["{{path}}".to_owned()],
        env: vec![],
    })
}
