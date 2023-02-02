use crate::error::{DisplayError, DisplayResult, Error, FormatCode::ScriptType as TypeCode};
use crate::util::illegal_name;
use crate::util::impl_ser_by_to_string;
use fxhash::FxHashMap as HashMap;
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

const DEFAULT_WELCOME_MSG: &str = "{{#each content}}{{{this}}}
{{/each}}";

const SHELL_WELCOME_MSG: &str = "# [HS_HELP]: Help message goes here...
# [HS_ENV]: VAR -> Description for env var `VAR` goes here
# [HS_ENV_HELP]: VAR2 -> Description for `VAR2` goes here, BUT won't be recorded

set -eu
{{#if birthplace_in_home}}
cd ~/{{birthplace_rel}}
{{else}}
cd {{birthplace}}
{{/if}}
{{#each content}}{{{this}}}
{{/each}}";

const JS_WELCOME_MSG: &str = "// [HS_HELP]: Help message goes here...
// [HS_ENV]: VAR -> Description for env var `VAR` goes here
// [HS_ENV_HELP]: VAR2 -> Description for `VAR2` goes here, BUT won't be recorded

process.chdir(require('os').homedir());
{{#if birthplace_in_home}}
process.chdir(process.env.HOME);
process.chdir('{{birthplace_rel}}');
{{else}}
process.chdir('{{birthplace}}');
{{/if}}
let spawn = require('child_process').spawnSync;
spawn('test', [], { stdio: 'inherit' });

let writeFile = require('fs').writeFileSync;
writeFile('/dev/null', 'some content');

{{#each content}}{{{this}}}
{{/each}}";

const TMUX_WELCOME_MSG: &str = "# [HS_HELP]: Help message goes here...
# [HS_ENV]: VAR -> Description for env var `VAR` goes here
# [HS_ENV_HELP]: VAR2 -> Description for `VAR2` goes here, BUT won't be recorded

NAME=${NAME/./_}
tmux has-session -t=$NAME
if [ $? = 0 ]; then
    echo attach to existing session
    tmux -2 attach-session -t $NAME
    exit
fi

set -eu
{{#if birthplace_in_home}}
cd ~/{{birthplace_rel}}
{{else}}
cd {{birthplace}}
{{/if}}
tmux new-session -s $NAME -d \"{{{content.0}}}; $SHELL\" || exit 1
tmux split-window -h \"{{{content.1}}}; $SHELL\"
{{#if content.2}}tmux split-window -v \"{{{content.2}}}; $SHELL\"
{{/if}}
tmux -2 attach-session -d";

const RB_WELCOME_MSG: &str = "# [HS_HELP]: Help message goes here...
# [HS_ENV]: VAR -> Description for env var `VAR` goes here
# [HS_ENV_HELP]: VAR2 -> Description for `VAR2` goes here, BUT won't be recorded
{{#if birthplace_in_home}}
Dir.chdir(\"#{ENV['HOME']}/{{birthplace_rel}}\")
{{else}}
Dir.chdir(\"{{birthplace}}\")
{{/if}}
{{#each content}}{{{this}}}
{{/each}}";

const RB_CD_WELCOME_MSG: &str =
"{{#if birthplace_in_home}}BASE = \"#{ENV['HOME']}/{{birthplace_rel}}\"
{{else}}BASE = '{{birthplace}}'
{{/if}}
require File.realpath(\"#{ENV['HS_HOME']}/util/common.rb\")
require 'set'

def cd(dir)
  File.open(HS_ENV.env_var(:source), 'w') do |file|
    file.write(\"cd #{BASE}/#{dir}\")
  end
  exit
end

cd(ARGV[0]) if ARGV != []

Dir.chdir(BASE)
dirs_set = Dir.entries('.').select do |c|
  !c.start_with?('.') && File.directory?(c)
end.to_set
dirs_set.add('.')

history_arr = HS_ENV.do_hs(\"history show =#{HS_ENV.env_var(:name)}!\", false).lines.map(&:strip).reject(&:empty?)

history_arr = history_arr.select do |d|
  if dirs_set.include?(d)
    dirs_set.delete(d)
    true
  else
    false
  end
end

require_relative \"#{ENV['HS_HOME']}/util/selector.rb\"
selector = Selector.new
selector.load(history_arr + dirs_set.to_a)
is_dot = false
selector.register_keys('.', lambda { |_, _|
  is_dot = true
}, msg: 'go to \".\"')

dir = begin
  content = selector.run.content
  if is_dot
    '.'
  else
    content
  end
rescue Selector::Empty
  warn 'empty'
  exit 1
rescue Selector::Quit
  warn 'quit'
  exit
end

HS_ENV.do_hs(\"run --dummy =#{HS_ENV.env_var(:name)}! #{dir}\", false)
cd(dir)";

const RB_TRAVERSE_WELCOME_MSG: &str = "# [HS_HELP]: Help message goes here...
# [HS_ENV]: VAR -> Description for env var `VAR` goes here
# [HS_ENV_HELP]: VAR2 -> Description for `VAR2` goes here, BUT won't be recorded

def directory_tree(path)
  files = []
  Dir.foreach(path) do |entry|
    next if ['..', '.'].include?(entry)

    full_path = File.join(path, entry)
    if File.directory?(full_path)
      directory_tree(full_path).each do |f|
        files.push(f)
      end
    else
      files.push(full_path)
    end
  end
  files
end
{{#if birthplace_in_home}}
Dir.chdir(\"#{ENV['HOME']}/{{birthplace_rel}}\")
{{else}}
Dir.chdir(\"{{birthplace}}\")
{{/if}}
directory_tree('.').each do |full_path|
  {{#each content}}{{{this}}}
  {{else}} # TODO{{/each}}
end";

#[derive(Clone, Display, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct ScriptType(String);
impl ScriptType {
    pub fn new_unchecked(s: String) -> Self {
        ScriptType(s)
    }
}
impl AsRef<str> for ScriptType {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl FromStr for ScriptType {
    type Err = DisplayError;
    fn from_str(s: &str) -> DisplayResult<Self> {
        if illegal_name(s) {
            log::error!("類型格式不符：{}", s);
            return TypeCode.to_display_res(s.to_owned());
        }
        Ok(ScriptType(s.to_string()))
    }
}
impl Default for ScriptType {
    fn default() -> Self {
        ScriptType("sh".to_string())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScriptFullType {
    pub ty: ScriptType,
    pub sub: Option<ScriptType>,
}
impl FromStr for ScriptFullType {
    type Err = DisplayError;
    fn from_str(s: &str) -> DisplayResult<Self> {
        if let Some((first, second)) = s.split_once("/") {
            Ok(ScriptFullType {
                ty: first.parse()?,
                sub: Some(second.parse()?),
            })
        } else {
            Ok(ScriptFullType {
                ty: s.parse()?,
                sub: None,
            })
        }
    }
}
impl Default for ScriptFullType {
    fn default() -> Self {
        Self {
            ty: ScriptType::default(),
            sub: None,
        }
    }
}
impl_ser_by_to_string!(ScriptFullType);

pub trait AsScriptFullTypeRef {
    fn get_ty(&self) -> &ScriptType;
    fn get_sub(&self) -> Option<&ScriptType>;
    fn display<'a>(&'a self) -> DisplayTy<'a, Self> {
        DisplayTy(self)
    }
    fn fmt(&self, w: &mut Formatter<'_>) -> FmtResult {
        if let Some(sub) = &self.get_sub() {
            write!(w, "{}/{}", self.get_ty(), sub)
        } else {
            write!(w, "{}", self.get_ty())
        }
    }
}
impl Display for ScriptFullType {
    fn fmt(&self, w: &mut Formatter<'_>) -> FmtResult {
        AsScriptFullTypeRef::fmt(self, w)
    }
}

impl AsScriptFullTypeRef for ScriptFullType {
    fn get_ty(&self) -> &ScriptType {
        &self.ty
    }
    fn get_sub(&self) -> Option<&ScriptType> {
        self.sub.as_ref()
    }
}

impl<'a> AsScriptFullTypeRef for (&'a ScriptType, Option<&'a ScriptType>) {
    fn get_ty(&self) -> &ScriptType {
        self.0
    }
    fn get_sub(&self) -> Option<&ScriptType> {
        self.1
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScriptTypeConfig {
    pub ext: Option<String>,
    pub color: String,
    pub cmd: Option<String>,
    args: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    env: HashMap<String, String>,
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
    ($(( $name:literal, $tmpl:ident, $conf:expr, [ $($sub:literal: $sub_tmpl:ident),* ] )),*) => {
        pub fn get_default_template<T: AsScriptFullTypeRef>(ty: &T) -> &'static str {
            match (ty.get_ty().as_ref(), ty.get_sub().map(|s| s.as_ref())) {
                $(
                    $(
                        ($name, Some($sub)) => $sub_tmpl,
                    )*
                    ($name, _) => $tmpl,
                )*
                _ => DEFAULT_WELCOME_MSG
            }
        }
        pub fn iter_default_templates() -> impl ExactSizeIterator<Item = (ScriptFullType, &'static str)> {
            let arr = [$(
                (ScriptFullType{ ty: ScriptType($name.to_owned()), sub: None }, $tmpl),
                $(
                    (ScriptFullType{ ty: ScriptType($name.to_owned()), sub: Some(ScriptType($sub.to_owned())) }, $sub_tmpl),
                )*
            )*];
            arr.into_iter()
        }
        fn iter_default_configs() -> impl ExactSizeIterator<Item = (ScriptType, ScriptTypeConfig)> {
            let arr = [$( (ScriptType($name.to_owned()), $conf), )*];
            arr.into_iter()
        }
    };
}

fn gen_map(arr: &[(&str, &str)]) -> HashMap<String, String> {
    arr.iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

create_default_types! {
    ("sh", SHELL_WELCOME_MSG, ScriptTypeConfig {
        ext: Some("sh".to_owned()),
        color: "bright magenta".to_owned(),
        cmd: Some("bash".to_owned()),
        args: vec!["{{path}}".to_owned()],
        env: Default::default()
    }, []),
    ("tmux", TMUX_WELCOME_MSG, ScriptTypeConfig {
        ext: Some("sh".to_owned()),
        color: "white".to_owned(),
        cmd: Some("bash".to_owned()),
        args: vec!["{{path}}".to_owned()],
        env: Default::default(),
    }, []),
    ("js", JS_WELCOME_MSG, ScriptTypeConfig {
        ext: Some("js".to_owned()),
        color: "bright cyan".to_owned(),
        cmd: Some("node".to_owned()),
        args: vec!["{{path}}".to_owned()],
        env: gen_map(&[(
            "NODE_PATH",
            "{{{home}}}/node_modules",
        )]),
    }, []),
    ("js-i", JS_WELCOME_MSG, ScriptTypeConfig {
        ext: Some("js".to_owned()),
        color: "bright cyan".to_owned(),
        cmd: Some("node".to_owned()),
        args: vec!["-i".to_owned(), "-e".to_owned(), "{{{content}}}".to_owned()],
        env: gen_map(&[(
            "NODE_PATH",
            "{{{home}}}/node_modules",
        )]),
    }, []),
    ("rb", RB_WELCOME_MSG, ScriptTypeConfig {
        ext: Some("rb".to_owned()),
        color: "bright red".to_owned(),
        cmd: Some("ruby".to_owned()),
        args: vec!["{{path}}".to_owned()],
        env: Default::default(),
    }, ["traverse": RB_TRAVERSE_WELCOME_MSG, "cd": RB_CD_WELCOME_MSG]),
    ("txt", DEFAULT_WELCOME_MSG, ScriptTypeConfig {
        ext: None,
        color: "bright black".to_owned(),
        cmd: Some("cat".to_owned()),
        args: vec!["{{path}}".to_owned()],
        env: Default::default(),
    }, [])
}

/// 因為沒辦法直接對 AsScriptFullTypeRef 實作 Display 不得不多包一層…
pub struct DisplayTy<'a, U: ?Sized>(pub &'a U);
impl<'a, U: AsScriptFullTypeRef> Display for DisplayTy<'a, U> {
    fn fmt(&self, w: &mut Formatter<'_>) -> FmtResult {
        self.0.fmt(w)
    }
}
