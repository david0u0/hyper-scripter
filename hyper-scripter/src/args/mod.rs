use crate::config::{Alias, Config, PromptLevel};
use crate::error::Result;
use crate::list::Grouping;
use crate::path;
use crate::query::{EditQuery, ListQuery, RangeQuery, ScriptQuery};
use crate::script::ScriptName;
use crate::script_type::ScriptType;
use crate::tag::TagFilter;
use crate::Either;
use serde::Serialize;
use std::path::PathBuf;
use structopt::clap::AppSettings::{
    self, AllArgsOverrideSelf, AllowExternalSubcommands, AllowLeadingHyphen, DisableHelpFlags,
    DisableHelpSubcommand, DisableVersion, Hidden, TrailingVarArg,
};
use structopt::StructOpt;

mod completion;
pub use completion::*;
mod tags;
pub use tags::*;
mod types;
pub use types::*;

const NO_FLAG_SETTINGS: &[AppSettings] = &[
    AllowLeadingHyphen,
    DisableHelpFlags,
    TrailingVarArg,
    DisableHelpSubcommand,
    DisableVersion,
    AllowExternalSubcommands,
];

#[derive(StructOpt, Debug, Serialize)]
pub struct RootArgs {
    #[structopt(short = "H", long, help = "Path to hyper script home")]
    pub hs_home: Option<String>,
    #[structopt(long, hidden = true)]
    pub dump_args: bool,
    #[structopt(long, help = "Do not record history")]
    pub no_trace: bool,
    #[structopt(
        long,
        conflicts_with = "no-trace",
        help = "Do not affect script time (but will still record history)"
    )]
    pub humble: bool,
    #[structopt(short = "A", long, global = true, help = "Show scripts NOT within recent days", conflicts_with_all = &["all", "timeless"])]
    pub archaeology: bool,
    #[structopt(long)]
    pub no_alias: bool, // NOTE: no-alias 的判斷其實存在於 structopt 之外，寫在這裡只是為了生成幫助訊息
    #[structopt(
        short,
        long,
        global = true,
        parse(try_from_str),
        conflicts_with = "all",
        number_of_values = 1,
        help = "Filter by tags, e.g. `all,^mytag`"
    )]
    pub filter: Vec<TagFilter>,
    #[structopt(
        long,
        conflicts_with = "all",
        number_of_values = 1,
        help = "Toggle named filter temporarily"
    )]
    pub toggle: Vec<String>, // TODO: new type?
    #[structopt(
        short,
        long,
        global = true,
        conflicts_with = "recent",
        help = "Shorthand for `-f=all,^removed --timeless`"
    )]
    all: bool,
    #[structopt(long, global = true, help = "Show scripts within recent days.")]
    pub recent: Option<u32>,
    #[structopt(
        long,
        global = true,
        help = "Show scripts of all time.",
        conflicts_with = "recent"
    )]
    pub timeless: bool,
    #[structopt(long, possible_values(&["never", "always", "smart", "on-multi-fuzz"]), help = "Prompt level of fuzzy finder.")]
    pub prompt_level: Option<PromptLevel>,
}

#[derive(StructOpt, Debug, Serialize)]
#[structopt(settings = &[AllowLeadingHyphen, AllArgsOverrideSelf])]
pub struct Root {
    #[structopt(skip = false)]
    #[serde(skip)]
    is_from_alias: bool,
    #[structopt(flatten)]
    pub root_args: RootArgs,
    #[structopt(subcommand)]
    pub subcmd: Option<Subs>,
}

#[derive(StructOpt, Debug, Serialize)]
pub enum AliasSubs {
    #[structopt(external_subcommand)]
    Other(Vec<String>),
}
#[derive(StructOpt, Debug, Serialize)]
#[structopt(settings = NO_FLAG_SETTINGS)]
pub struct AliasRoot {
    #[structopt(flatten)]
    pub root_args: RootArgs,
    #[structopt(subcommand)]
    pub subcmd: Option<AliasSubs>,
}
impl AliasRoot {
    fn find_alias<'a>(&'a self, conf: &'a Config) -> Option<(&'a Alias, &'a [String])> {
        match &self.subcmd {
            None => None,
            Some(AliasSubs::Other(v)) => {
                let first = v.first().unwrap().as_str();
                if let Some(alias) = conf.alias.get(first) {
                    log::info!("別名 {} => {:?}", first, alias);
                    Some((alias, v))
                } else {
                    None
                }
            }
        }
    }
    pub fn expand_alias<'a, T: 'a + AsRef<str>>(
        &'a self,
        args: &'a [T],
        conf: &'a Config,
    ) -> Option<impl Iterator<Item = &'a str>> {
        if let Some((alias, remaining_args)) = self.find_alias(conf) {
            let base_len = args.len() - remaining_args.len();
            let base_args = args.iter().take(base_len).map(AsRef::as_ref);
            let after_args = alias.after.iter().map(AsRef::as_ref);
            let remaining_args = remaining_args[1..].iter().map(AsRef::as_ref);
            let new_args = base_args.chain(after_args).chain(remaining_args);

            // log::trace!("新的參數為 {:?}", new_args);
            Some(new_args)
        } else {
            None
        }
    }
}

#[derive(StructOpt, Debug, Serialize)]
#[structopt(settings = &[AllArgsOverrideSelf])]
pub enum Subs {
    #[structopt(external_subcommand)]
    Other(Vec<String>),
    #[structopt(
        about = "Prints this message, the help of the given subcommand(s), or a script's help message."
    )]
    #[structopt(
        about = "Prints this message, the help of the given subcommand(s), or a script's help message.",
        setting = AllowLeadingHyphen
    )]
    Help { args: Vec<String> },
    #[structopt(setting = Hidden, about = "Print the help message of env variables")]
    EnvHelp {
        #[structopt(default_value = "-", parse(try_from_str))]
        script_query: ScriptQuery,
    },
    #[structopt(setting = Hidden)]
    LoadUtils,
    #[structopt(about = "Migrate the database")]
    Migrate,
    #[structopt(about = "Edit hyper script", settings = &[AllowLeadingHyphen, TrailingVarArg])]
    Edit {
        #[structopt(
            long,
            short = "T",
            parse(try_from_str),
            help = "Type of the script, e.g. `sh`"
        )]
        ty: Option<ScriptType>,
        #[structopt(long, short)]
        no_template: bool,
        #[structopt(long, short)]
        tags: Option<TagFilter>,
        #[structopt(long, help = "Create script without invoking the editor")]
        fast: bool,
        #[structopt(parse(try_from_str), default_value = ".")]
        edit_query: EditQuery,
        content: Vec<String>,
    },
    #[structopt(about = "Manage alias", settings = NO_FLAG_SETTINGS)]
    Alias {
        #[structopt(long, conflicts_with_all = &["before", "after"])]
        short: bool,
        #[structopt(
            long,
            short,
            requires = "before",
            conflicts_with = "after",
            help = "Unset an alias."
        )]
        unset: bool,
        before: Option<String>,
        after: Vec<String>,
    },

    #[structopt(about = "Run the script", settings = NO_FLAG_SETTINGS)]
    Run {
        #[structopt(long, help = "Add a dummy run history instead of actually running it")]
        dummy: bool,
        #[structopt(long, short)]
        repeat: Option<u64>,
        #[structopt(long, short, help = "Use arguments from last run")]
        previous_args: bool,
        #[structopt(long, short, requires = "previous-args")]
        dir: Option<PathBuf>,
        #[structopt(default_value = "-", parse(try_from_str))]
        script_query: ScriptQuery,
        #[structopt(help = "Command line args to pass to the script")]
        args: Vec<String>,
    },
    #[structopt(about = "Execute the script query and get the exact file")]
    Which {
        #[structopt(default_value = "-", parse(try_from_str))]
        script_query: ScriptQuery,
    },
    #[structopt(about = "Print the script to standard output")]
    Cat {
        #[structopt(default_value = "-", parse(try_from_str))]
        script_query: ScriptQuery,
    },
    #[structopt(about = "Remove the script")]
    RM {
        #[structopt(parse(try_from_str), required = true, min_values = 1)]
        queries: Vec<ListQuery>,
        #[structopt(
            long,
            help = "Actually remove scripts, rather than hiding them with tag."
        )]
        purge: bool,
    },
    #[structopt(about = "List hyper scripts")]
    LS(List),
    #[structopt(about = "Manage script types")]
    Types(Types),
    #[structopt(about = "Copy the script to another one")]
    CP {
        #[structopt(long, short)]
        tags: Option<TagFilter>,
        origin: ScriptQuery,
        new: ScriptName,
    },
    #[structopt(about = "Move the script to another one")]
    MV {
        #[structopt(
            long,
            short = "T",
            parse(try_from_str),
            help = "Type of the script, e.g. `sh`"
        )]
        ty: Option<ScriptType>,
        #[structopt(short, long)]
        tags: Option<TagFilter>,
        #[structopt(parse(try_from_str))]
        origin: ListQuery,
        new: Option<ScriptName>,
    },
    #[structopt(
        about = "Manage script tags. If a tag filter is given, store it to config, otherwise show tag information."
    )]
    Tags(Tags),
    #[structopt(about = "Manage script history")]
    History {
        #[structopt(subcommand)]
        subcmd: History,
    },
}

#[derive(StructOpt, Debug, Serialize)]
pub enum History {
    RM {
        #[structopt(parse(try_from_str), required = true, min_values = 1)]
        queries: Vec<ListQuery>,
        range: RangeQuery,
    },
    // TODO: 好想把它寫在 history rm 裡面...
    #[structopt(
        name = "rm-id",
        about = "Remove an event by it's id.\nUseful if you want to keep those illegal arguments from polluting the history."
    )]
    RMID { event_id: u64 },
    #[structopt(about = "Humble an event by it's id")]
    Humble { event_id: u64 },
    Show {
        #[structopt(default_value = "-", parse(try_from_str))]
        queries: Vec<ListQuery>,
        #[structopt(short, long, default_value = "10")]
        limit: u32,
        #[structopt(long)]
        with_name: bool,
        #[structopt(short, long, default_value = "0")]
        offset: u32,
        #[structopt(short, long)]
        dir: Option<PathBuf>,
    },
    Neglect {
        #[structopt(parse(try_from_str), required = true, min_values = 1)]
        queries: Vec<ListQuery>,
    },
    #[structopt( settings = NO_FLAG_SETTINGS)]
    Amend {
        event_id: u64,
        #[structopt(help = "Command line args to pass to the script")]
        args: Vec<String>,
    },
    Tidy {
        #[structopt(parse(try_from_str), required = true, min_values = 1)]
        queries: Vec<ListQuery>,
    },
}

#[derive(StructOpt, Debug, Serialize, Default)]
#[structopt(settings = &[AllArgsOverrideSelf])]
pub struct List {
    // TODO: 滿滿的其它排序/篩選選項
    #[structopt(short, long, help = "Show verbose information.")]
    pub long: bool,
    #[structopt(long, possible_values(&["tag", "tree", "none"]), default_value = "tag", help = "Grouping style.")]
    pub grouping: Grouping,
    #[structopt(long, help = "No color and other decoration.")]
    pub plain: bool,
    #[structopt(long, help = "Show file path to the script.", conflicts_with = "long")]
    pub file: bool,
    #[structopt(long, help = "Show name of the script.", conflicts_with = "long")]
    pub name: bool,
    #[structopt(parse(try_from_str))]
    pub queries: Vec<ListQuery>,
}

fn set_home(p: &Option<String>, create_on_missing: bool) -> Result {
    path::set_home(p.as_ref(), create_on_missing)?;
    Config::init()
}

fn print_help<S: AsRef<str>>(cmds: impl IntoIterator<Item = S>) -> Result {
    // 從 clap 的 parse_help_subcommand 函式抄的，不曉得有沒有更好的做法
    let c: structopt::clap::App = Root::clap();
    let mut clap = &c;
    let mut had_found = false;
    for cmd in cmds {
        let cmd = cmd.as_ref();
        if let Some(c) = clap.p.subcommands.iter().find(|s| &*s.p.meta.name == cmd) {
            clap = c;
            had_found = true;
        } else if !had_found {
            return Ok(());
        }
    }
    clap.clone().print_help()?;
    println!();
    std::process::exit(0);
}

fn handle_alias_args(args: Vec<String>) -> Result<Root> {
    if args.iter().any(|s| s == "--no-alias") {
        log::debug!("不使用別名！"); // NOTE: --no-alias 的判斷存在於 structopt 之外！
        let root = Root::from_iter(args);
        return Ok(root);
    }
    match AliasRoot::from_iter_safe(&args) {
        Ok(alias_root) => {
            log::info!("別名命令行物件 {:?}", alias_root);
            set_home(&alias_root.root_args.hs_home, true)?;
            let mut root = match alias_root.expand_alias(&args, Config::get()) {
                Some(new_args) => Root::from_iter(new_args),
                None => Root::from_iter(&args),
            };
            root.is_from_alias = true;
            Ok(root)
        }
        Err(e) => {
            log::warn!("解析別名參數出錯：{}", e); // NOTE: 不要讓這個錯誤傳上去，而是讓它掉入 Root::from_iter 中再來報錯
            Root::from_iter(args);
            unreachable!()
        }
    }
}

impl Root {
    /// 若帶了 --no-alias 選項，或是補全模式，我們可以把設定腳本之家（以及載入設定檔）的時間再推遲
    /// 在補全模式中意義重大，因為使用者可能會用 -H 指定別的腳本之家
    pub fn set_home_unless_from_alias(&self, create_on_missing: bool) -> Result {
        if !self.is_from_alias {
            set_home(&self.root_args.hs_home, create_on_missing)?;
        }
        Ok(())
    }
    pub fn sanitize_flags(&mut self) {
        if self.root_args.all {
            self.root_args.timeless = true;
            self.root_args.filter = vec!["all,^removed".parse().unwrap()];
        }
    }
    pub fn sanitize(&mut self) -> Result {
        match &mut self.subcmd {
            Some(Subs::Other(args)) => {
                let args =
                    std::array::IntoIter::new(["hs", "run"]).chain(args.iter().map(|s| s.as_str()));
                self.subcmd = Some(Subs::from_iter(args));
                log::info!("執行模式 {:?}", self.subcmd);
            }
            Some(Subs::Help { args }) => {
                print_help(args.iter())?;
            }
            Some(Subs::Tags(tags)) => {
                tags.sanitize();
            }
            Some(Subs::Types(types)) => {
                types.sanitize();
            }
            None => {
                log::info!("無參數模式");
                self.subcmd = Some(Subs::Edit {
                    edit_query: EditQuery::default(),
                    ty: None,
                    content: vec![],
                    tags: None,
                    fast: false,
                    no_template: false,
                });
            }
            _ => (),
        }
        self.sanitize_flags();
        Ok(())
    }
}

pub fn handle_args(args: Vec<String>) -> Result<Either<Root, Completion>> {
    if let Some(completion) = Completion::from_args(&args) {
        return Ok(Either::Two(completion));
    }
    let mut root = handle_alias_args(args)?;
    log::debug!("命令行物件：{:?}", root);

    root.sanitize()?;
    Ok(Either::One(root))
}

#[cfg(test)]
mod test {
    use super::*;
    fn build_args<'a>(args: &'a str) -> Root {
        let v: Vec<_> = std::iter::once("hs")
            .chain(args.split(' '))
            .map(|s| s.to_owned())
            .collect();
        match handle_args(v).unwrap() {
            Either::One(root) => root,
            _ => panic!(),
        }
    }
    #[test]
    #[ignore = "structopt bug"]
    fn test_strange_set_alias() {
        let args = build_args("alias trash -f removed");
        assert_eq!(args.root_args.filter, vec![]);
        match &args.subcmd {
            Some(Subs::Alias {
                unset,
                short,
                after,
                before: Some(before),
            }) => {
                assert_eq!(*unset, false);
                assert_eq!(*short, false);
                assert_eq!(before, "trash");
                assert_eq!(after, &["-f", "removed"]);
            }
            _ => panic!("{:?} should be alias...", args),
        }
    }
    #[test]
    fn test_strange_alias() {
        let args = build_args("-f e e -t e something -T e");
        assert_eq!(args.root_args.filter, vec!["e".parse().unwrap()]);
        assert_eq!(args.root_args.all, false);
        match &args.subcmd {
            Some(Subs::Edit {
                edit_query,
                tags,
                ty,
                ..
            }) => {
                assert_eq!(edit_query, &"something".parse().unwrap());
                assert_eq!(tags, &Some("e".parse().unwrap()));
                assert_eq!(ty, &Some("e".parse().unwrap()));
            }
            _ => {
                panic!("{:?} should be edit...", args);
            }
        }

        let args = build_args("la -l");
        assert_eq!(args.root_args.filter, vec!["all,^removed".parse().unwrap()]);
        assert_eq!(args.root_args.all, true);
        match &args.subcmd {
            Some(Subs::LS(opt)) => {
                assert_eq!(opt.long, true);
                assert_eq!(opt.queries.len(), 0);
            }
            _ => {
                panic!("{:?} should be edit...", args);
            }
        }
    }
    #[test]
    fn test_default_run_tags() {
        // TODO
    }
    #[test]
    fn test_external_run_tags() {
        let args = build_args("-f test --dummy -r 42 =script -a --");
        assert_eq!(args.root_args.filter, vec!["test".parse().unwrap()]);
        assert_eq!(args.root_args.all, false);
        match args.subcmd {
            Some(Subs::Run {
                dummy: true,
                previous_args: false,
                repeat: Some(42),
                dir: None,
                script_query,
                args,
            }) => {
                assert_eq!(script_query, "=script".parse().unwrap());
                assert_eq!(args, vec!["-a", "--"]);
            }
            _ => {
                panic!("{:?} should be run...", args);
            }
        }

        let args = build_args("-f test --dump-args tags --name myname +mytag");
        assert_eq!(args.root_args.filter, vec!["test".parse().unwrap()]);
        assert_eq!(args.root_args.all, false);
        assert!(args.root_args.dump_args);
        match args.subcmd {
            Some(Subs::Tags(Tags {
                subcmd: Some(TagsSubs::Set { name, content }),
            })) => {
                assert_eq!(name, Some("myname".to_owned()));
                assert_eq!(content, "+mytag".parse().unwrap());
            }
            _ => {
                panic!("{:?} should be tags...", args);
            }
        }
    }
}
