use crate::config::{Alias, Config};
use crate::error::Result;
use crate::path;
use crate::query::{EditQuery, FilterQuery, ListQuery, ScriptQuery};
use crate::script::ScriptName;
use crate::script_type::ScriptType;
use crate::tag::TagFilter;
use structopt::clap::AppSettings::{
    self, AllArgsOverrideSelf, AllowExternalSubcommands, AllowLeadingHyphen, DisableHelpFlags,
    DisableHelpSubcommand, DisableVersion, TrailingVarArg,
};
use structopt::StructOpt;

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
        #[derive(StructOpt, Debug)]
        #[structopt(settings = &[AllowLeadingHyphen, AllArgsOverrideSelf])]
        pub struct Root {
            #[structopt(long)]
            pub no_alias: bool,
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

            #[structopt(subcommand)]
            pub $sub: Option<$sub_type>,
        }
    };
}

mod alias_mod {
    use super::{AllArgsOverrideSelf, AllowLeadingHyphen, StructOpt, TagFilter};
    #[derive(StructOpt, Debug)]
    pub enum Subs {
        #[structopt(external_subcommand)]
        Other(Vec<String>),
    }
    def_root! {
        subcmd: Subs
    }
}

def_root! {
    subcmd: Subs
}

#[derive(StructOpt, Debug)]
#[structopt(settings = &[AllArgsOverrideSelf])]
pub enum Subs {
    #[structopt(external_subcommand)]
    Other(Vec<String>),
    #[structopt(
        about = "Prints this message, the help of the given subcommand(s), or a script's help message."
    )]
    Help { args: Vec<String> },
    #[structopt(setting = AppSettings::Hidden)]
    LoadUtils,
    #[structopt(about = "Edit hyper script")]
    Edit {
        #[structopt(
            long,
            short,
            parse(try_from_str),
            help = "Category of the script, e.g. `sh`"
        )]
        category: Option<ScriptType>,
        #[structopt(long, short)]
        no_template: bool,
        #[structopt(long, short)]
        tags: Option<TagFilter>,
        #[structopt(
            long,
            requires("content"),
            help = "create script without invoking the editor"
        )]
        fast: bool,
        #[structopt(parse(try_from_str), default_value = ".")]
        edit_query: EditQuery,
        content: Option<String>,
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
        #[structopt(default_value = "-", parse(try_from_str))]
        script_query: ScriptQuery,
        #[structopt(help = "Command line args to pass to the script")]
        args: Vec<String>,
    },
    #[structopt(about = "Repeat the script", settings = NO_FLAG_SETTINGS)]
    Repeat {
        #[structopt(about = "The times you want to repeat the script")]
        times: u64,
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
        #[structopt(parse(try_from_str))]
        origin: ScriptQuery,
        new: ScriptName,
    },
    #[structopt(about = "Move the script to another one")]
    MV {
        #[structopt(
            long,
            short,
            parse(try_from_str),
            help = "Category of the script, e.g. `sh`"
        )]
        category: Option<ScriptType>,
        #[structopt(short, long)]
        tags: Option<TagFilter>,
        #[structopt(parse(try_from_str))]
        origin: ScriptQuery,
        new: Option<ScriptName>,
    },
    #[structopt(
        about = "Manage script tags. If a tag filter is given, store it to config, otherwise show tag information."
    )]
    Tags {
        #[structopt(parse(try_from_str))]
        tag_filter: Option<FilterQuery>,
    },
}

#[derive(StructOpt, Debug)]
#[structopt(settings = &[AllArgsOverrideSelf])]
pub struct List {
    // TODO: 滿滿的其它排序/篩選選項
    #[structopt(short, long, help = "Show verbose information.")]
    pub long: bool,
    #[structopt(long, possible_values(&["tag", "tree", "none"]), default_value = "tag", help = "Grouping style.")]
    pub grouping: String,
    #[structopt(long, help = "No color and other decoration.")]
    pub plain: bool,
    #[structopt(
        long,
        help = "Show file path to the script.",
        conflicts_with("long"),
        overrides_with("name")
    )]
    pub file: bool,
    #[structopt(
        long,
        help = "Show only name of the script.",
        conflicts_with("long"),
        overrides_with("file")
    )]
    pub name: bool,
    #[structopt(parse(try_from_str))]
    pub queries: Vec<ListQuery>,
}

fn set_home(p: &Option<String>) -> Result {
    match p {
        Some(p) => path::set_home(p),
        None => path::set_home_from_sys(),
    }
}

fn find_alias<'a>(root: &'a alias_mod::Root) -> Result<Option<(&'a str, &'static Alias)>> {
    match &root.subcmd {
        Some(alias_mod::Subs::Other(v)) => {
            let first = v.first().unwrap().as_str();
            let conf = Config::get()?;
            if let Some(alias) = conf.alias.get(first) {
                Ok(Some((first, alias)))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

pub fn print_help<S: AsRef<str>>(cmds: impl IntoIterator<Item = S>) -> Result {
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
    println!("");
    std::process::exit(0);
}

fn handle_alias_args(args: &[String]) -> Result<Root> {
    match alias_mod::Root::from_iter_safe(args) {
        Ok(alias_root) => {
            log::trace!("別名命令行物件 {:?}", alias_root);
            if alias_root.no_alias {
                log::debug!("不使用別名！");
            } else {
                set_home(&alias_root.hs_home)?;
                if let Some((before, alias)) = find_alias(&alias_root)? {
                    log::info!("別名 {} => {:?}", before, alias);
                    let mut new_args: Vec<&str> = vec![];
                    let mut arg_iter = args.iter();
                    while let Some(arg) = arg_iter.next() {
                        if before == arg {
                            new_args.extend(alias.after.iter().map(|s| s.as_str()));
                            new_args.extend(arg_iter.map(|s| s.as_str()));
                            break;
                        } else {
                            new_args.push(arg);
                        }
                    }
                    log::trace!("新的參數為 {:?}", new_args);
                    return Ok(Root::from_iter(new_args));
                }
            }
        }
        Err(e) => {
            log::warn!("解析別名參數出錯： {}", e);
        }
    };

    let root = Root::from_iter(args);
    set_home(&root.hs_home)?;
    Ok(root)
}

impl Root {
    pub fn sanitize(&mut self) -> Result {
        match &self.subcmd {
            Some(Subs::Other(args)) => {
                let run = Subs::Run {
                    script_query: args[0].parse()?,
                    args: args[1..args.len()].iter().map(|s| s.clone()).collect(),
                };
                log::info!("執行模式 {:?}", run);
                self.subcmd = Some(run);
            }
            None => {
                log::info!("無參數模式");
                self.subcmd = Some(Subs::Edit {
                    edit_query: EditQuery::default(),
                    category: None,
                    content: None,
                    tags: None,
                    fast: false,
                    no_template: false,
                });
            }
            _ => (),
        }
        if self.all {
            self.timeless = true;
            self.filter = vec!["all,^removed".parse().unwrap()];
        }
        Ok(())
    }
}

pub fn handle_args() -> Result<Root> {
    let args: Vec<_> = std::env::args().map(|s| s).collect();
    let mut root = handle_alias_args(&args)?;
    log::debug!("命令行物件：{:?}", root);

    root.sanitize()?;
    Ok(root)
}

// TODO: 單元測試！
