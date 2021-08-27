use crate::config::{Alias, Config, PromptLevel};
use crate::error::Result;
use crate::list::Grouping;
use crate::path;
use crate::query::{EditQuery, FilterQuery, ListQuery, RangeQuery, ScriptQuery};
use crate::script::ScriptName;
use crate::script_type::ScriptType;
use crate::tag::TagFilter;
use crate::Either;
use serde::Serialize;
use structopt::clap::AppSettings::{
    self, AllArgsOverrideSelf, AllowExternalSubcommands, AllowLeadingHyphen, DisableHelpFlags,
    DisableHelpSubcommand, DisableVersion, TrailingVarArg,
};
use structopt::StructOpt;

mod completion;
pub use completion::*;

const NO_FLAG_SETTINGS: &[AppSettings] = &[
    AllowLeadingHyphen,
    DisableHelpFlags,
    TrailingVarArg,
    DisableHelpSubcommand,
    DisableVersion,
    AllowExternalSubcommands,
];

macro_rules! def_root {
    ($sub:ident: $sub_type:ty) => {
        #[derive(StructOpt, Debug, Serialize)]
        #[structopt(settings = &[AllowLeadingHyphen, AllArgsOverrideSelf])]
        pub struct Root {
            #[structopt(skip = true)]
            pub home_is_set: bool,

            #[structopt(long, hidden = true)]
            pub dump_args: bool,
            #[structopt(long, global = true, help = "Do not record history")]
            pub no_trace: bool,
            #[structopt(short = "A", long, global = true, help = "Show scripts NOT within recent days", conflicts_with_all = &["all", "timeless"])]
            pub archaeology: bool,
            #[structopt(long, global = true)]
            pub no_alias: bool, // NOTE: no-alias 的判斷其實存在於 structopt 之外，寫在這裡只是為了生成幫助訊息
            #[structopt(short = "H", long, help = "Path to hyper script home")]
            pub hs_home: Option<String>,
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
            #[structopt(long, possible_values(&["never", "always", "smart"]), help = "Prompt level of fuzzy finder.")]
            pub prompt_level: Option<PromptLevel>,

            #[structopt(subcommand)]
            pub $sub: Option<$sub_type>,
        }
    };
}

mod alias_mod {
    use super::*;
    #[derive(StructOpt, Debug, Serialize)]
    pub enum Subs {
        #[structopt(external_subcommand)]
        Other(Vec<String>),
    }
    def_root! {
        subcmd: Subs
    }

    fn find_alias<'a>(root: &'a AliasRoot, conf: &'a Config) -> Option<(&'a Alias, &'a [String])> {
        match &root.subcmd {
            Some(alias_mod::Subs::Other(v)) => {
                let first = v.first().unwrap().as_str();
                if let Some(alias) = conf.alias.get(first) {
                    log::info!("別名 {} => {:?}", first, alias);
                    Some((alias, v))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    impl Root {
        pub fn expand_alias<'a, T: 'a + AsRef<str>>(
            &'a self,
            args: &'a [T],
            conf: &'a Config,
        ) -> Option<impl Iterator<Item = &'a str>> {
            if let Some((alias, remaining_args)) = find_alias(self, conf) {
                let base_len = args.len() - remaining_args.len();
                let base_args = args.iter().take(base_len).map(AsRef::as_ref);
                let after_args = alias.after.iter().map(AsRef::as_ref);
                let remaining_args = remaining_args.iter().map(AsRef::as_ref).skip(1);
                let new_args = base_args.chain(after_args).chain(remaining_args);

                // log::trace!("新的參數為 {:?}", new_args);
                Some(new_args)
            } else {
                None
            }
        }
    }
}
pub use alias_mod::Root as AliasRoot;

def_root! {
    subcmd: Subs
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
    #[structopt(about = "Print the help message of env variables")]
    EnvHelp {
        #[structopt(default_value = "-", parse(try_from_str))]
        script_query: ScriptQuery,
    },
    #[structopt(setting = AppSettings::Hidden)]
    LoadUtils,
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
        #[structopt(long, short, default_value = "1")]
        repeat: u64,
        #[structopt(long, short, help = "Use arguments from previous run")]
        previous_args: bool,
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
    Tags {
        #[structopt(short, long, conflicts_with = "switch-activated")]
        unset: Option<String>,
        #[structopt(short, long, conflicts_with = "unset")]
        switch_activated: Option<String>,

        #[structopt(parse(try_from_str), conflicts_with_all = &["unset", "switch-activated"])]
        tag_filter: Option<FilterQuery>,
    },
    #[structopt(about = "Manage script history")]
    History {
        #[structopt(subcommand)]
        subcmd: History,
    },
}

#[derive(StructOpt, Debug, Serialize)]
pub enum History {
    RM {
        #[structopt(parse(try_from_str))]
        script: ScriptQuery,
        range: RangeQuery,
    },
    // TODO: 好想把它寫在 history rm 裡面...
    #[structopt(
        name = "rm-id",
        about = "Remove history by the event's id\nUseful if you want to keep those illegal arguments from polluting the history."
    )]
    RMID { event_id: u64 }, // TODO: 測試
    Show {
        #[structopt(default_value = "-", parse(try_from_str))]
        script: ScriptQuery,
        #[structopt(short, long, default_value = "10")]
        limit: u32,
        #[structopt(long)]
        with_name: bool,
        #[structopt(short, long, default_value = "0")]
        offset: u32,
    },
    Neglect {
        #[structopt(parse(try_from_str), required = true, min_values = 1)]
        queries: Vec<ListQuery>,
    },
    #[structopt( settings = NO_FLAG_SETTINGS)] // TODO: 測試
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

fn set_home(p: &Option<String>) -> Result {
    path::set_home(p.as_ref())?;
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
    use structopt::clap::{Error as ClapError, ErrorKind::VersionDisplayed};
    if args.iter().any(|s| s == "--no-alias") {
        log::debug!("不使用別名！"); // NOTE: --no-alias 的判斷存在於 structopt 之外！
        let mut root = Root::from_iter(args);
        root.home_is_set = false;
        return Ok(root);
    }
    match AliasRoot::from_iter_safe(&args) {
        Ok(alias_root) => {
            log::trace!("別名命令行物件 {:?}", alias_root);
            set_home(&alias_root.hs_home)?;
            if let Some(new_args) = alias_root.expand_alias(&args, Config::get()) {
                // log::trace!("新的參數為 {:?}", new_args);
                return Ok(Root::from_iter(new_args));
            }
            let root = Root::from_iter(args);
            Ok(root)
        }
        Err(ClapError {
            kind: VersionDisplayed,
            ..
        }) => {
            // `from_iter_safe` 中已打印出版本，不再多做事，直接退出
            std::process::exit(0);
        }
        Err(e) => {
            log::warn!("解析別名參數出錯：{}", e); // NOTE: 不要讓這個錯誤傳上去，而是讓它掉入 Root::from_iter 中再來報錯
            Root::from_iter(args);
            unreachable!()
        }
    }
}

impl Root {
    /// 若帶了 --no-alias 選項，我們可以把設定腳本之家（以及載入設定檔）的時間再推遲
    pub fn set_home_unless_set(&self) -> Result {
        if !self.home_is_set {
            set_home(&self.hs_home)?;
        }
        Ok(())
    }
    pub fn sanitize_flags(&mut self) {
        if self.all {
            self.timeless = true;
            self.filter = vec!["all,^removed".parse().unwrap()];
        }
    }
    pub fn sanitize(&mut self) -> Result {
        match &self.subcmd {
            Some(Subs::Other(args)) => {
                let run = Subs::Run {
                    dummy: false,
                    previous_args: false,
                    repeat: 1,
                    script_query: args[0].parse()?,
                    args: args[1..args.len()].to_vec(),
                };
                log::info!("執行模式 {:?}", run);
                self.subcmd = Some(run);
            }
            Some(Subs::Help { args }) => {
                print_help(args.iter())?;
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
    if let Some(completion) = Completion::from_args(&args)? {
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
        assert_eq!(args.filter, vec![]);
        match &args.subcmd {
            Some(Subs::Alias {
                unset,
                before: Some(before),
                after,
            }) => {
                assert_eq!(*unset, false);
                assert_eq!(before, "trash");
                assert_eq!(after, &["-f", "removed"]);
            }
            _ => panic!("{:?} should be alias...", args),
        }
    }
    #[test]
    fn test_strange_alias() {
        let args = build_args("-f e e -t e something -T e");
        assert_eq!(args.filter, vec!["e".parse().unwrap()]);
        assert_eq!(args.all, false);
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
        assert_eq!(args.filter, vec!["all,^removed".parse().unwrap()]);
        assert_eq!(args.all, true);
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
}
