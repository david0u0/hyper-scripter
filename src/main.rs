use chrono::Utc;
use instant_scripter::error::{Contextabl, Error, Result};
use instant_scripter::history::History;
use instant_scripter::list::{fmt_list, ListOptions, ListPattern};
use instant_scripter::script::{AsScriptName, ScriptInfo, ScriptType};
use instant_scripter::tag::TagFilters;
use instant_scripter::{fuzzy, path, util};
use structopt::clap::AppSettings::{
    self, AllowLeadingHyphen, DisableHelpFlags, DisableHelpSubcommand, DisableVersion,
    TrailingVarArg,
};
use structopt::StructOpt;

const NO_FLAG_SETTINGS: &[AppSettings] = &[
    AllowLeadingHyphen,
    DisableHelpFlags,
    TrailingVarArg,
    DisableHelpSubcommand,
    DisableVersion,
];

#[derive(StructOpt, Debug)]
#[structopt(setting = AllowLeadingHyphen)]
struct Root {
    #[structopt(short = "p", long, help = "Path to instant script root")]
    is_path: Option<String>,
    #[structopt(short, long, parse(try_from_str), default_value = "all,-hide")]
    tags: TagFilters,
    #[structopt(short, long, help = "Shorthand for `-t=all`")]
    all: bool,
    #[structopt(subcommand)]
    subcmd: Option<Subs>,
}
#[derive(StructOpt, Debug)]
enum Subs {
    #[structopt(external_subcommand)]
    Other(Vec<String>),
    #[structopt(about = "Edit instant script", alias = "e")]
    Edit {
        #[structopt(short, long, help = "Don't do fuzzy search")]
        exact: bool,
        #[structopt(short, long, help = "The content for your script.")]
        content: Option<String>,
        script_name: Option<String>,
        #[structopt(
            long,
            short = "x",
            parse(try_from_str),
            help = "Executable type of the script, e.g. `sh`"
        )]
        executable: Option<ScriptType>,
    },
    #[structopt(about = "Run the script", settings = NO_FLAG_SETTINGS)]
    Run {
        #[structopt(default_value = "-")]
        script_name: String,
        #[structopt(help = "Command line args to pass to the script.")]
        args: Vec<String>,
    },
    #[structopt(about = "Print the script to standard output.")]
    Cat {
        #[structopt(default_value = "-")]
        script_name: String,
    },
    #[structopt(about = "Remove the script")]
    RM {
        #[structopt(short, long, help = "Don't do fuzzy search")]
        exact: bool,
        #[structopt(required = true, min_values = 1)]
        scripts: Vec<String>,
    },
    #[structopt(about = "List instant scripts", alias = "l")]
    LS(List),
    #[structopt(about = "Copy the script to another one")]
    CP {
        #[structopt(short, long, help = "Don't do fuzzy search")]
        exact: bool,
        origin: String,
        new: String,
    },
    #[structopt(about = "Move the script to another one")]
    MV {
        #[structopt(short, long, help = "Don't do fuzzy search")]
        exact: bool,
        #[structopt(
            long,
            short = "x",
            parse(try_from_str),
            help = "Executable type of the script, e.g. `sh`"
        )]
        executable: Option<ScriptType>,
        #[structopt(short, long)]
        tags: Option<TagFilters>,
        origin: String,
        new: Option<String>,
    },
    #[structopt(
        about = "Manage script tags. If a list of tag is given, set it as default, otherwise show tag information."
    )]
    Tags { tags: Option<TagFilters> },
}

impl Root {
    fn all(&self) -> bool {
        if self.all {
            return true;
        }
        if let Some(Subs::LS(List { all, .. })) = self.subcmd {
            all
        } else {
            false
        }
    }
}

#[derive(StructOpt, Debug)]
struct List {
    // TODO: 滿滿的其它排序/篩選選項
    #[structopt(short, long, help = "Show all scripts.")]
    all: bool,
    #[structopt(short, long, help = "Show verbose information.")]
    long: bool,
    #[structopt(parse(try_from_str))]
    pattern: Option<ListPattern>,
}

fn main() -> Result<()> {
    env_logger::init();
    let mut root = Root::from_args();
    log::debug!("命令行物件：{:?}", root);
    match &root.is_path {
        Some(is_path) => path::set_path(is_path)?,
        None => path::set_path_from_sys()?,
    }
    let mut hs = path::get_history().context("讀取歷史記錄失敗")?;
    if !root.all() {
        hs.filter_by_group(&root.tags);
    }
    main_inner(&mut root, hs)
}
fn get_info_mut<'b, 'a>(
    name: &str,
    history: &'b mut History<'a>,
    exact: bool,
) -> Result<Option<&'b mut ScriptInfo<'a>>> {
    log::trace!("開始尋找 `{}`, exact={}", name, exact);
    if name == "-" {
        log::trace!("找最新腳本");
        let latest = history.latest_mut();
        if latest.is_some() {
            Ok(latest)
        } else {
            Err(Error::Empty)
        }
    } else if name.starts_with("-") {
        return Err(Error::ScriptNameFormat(name.to_owned()));
    } else if exact {
        let name = name
            .clone() // TODO: Cow 優化
            .as_script_name()?;
        Ok(history.get_mut(&name))
    } else {
        fuzzy::fuzz_mut(name, history.iter_mut())
    }
}
fn get_info_mut_strict<'b, 'a>(
    name: &str,
    history: &'b mut History<'a>,
    exact: bool,
) -> Result<&'b mut ScriptInfo<'a>> {
    match get_info_mut(name, history, exact) {
        Err(e) => Err(e),
        Ok(None) => Err(Error::NoInfo(name.to_owned())),
        Ok(Some(info)) => Ok(info),
    }
}
fn main_inner<'a>(root: &mut Root, mut hs: History<'a>) -> Result<()> {
    let mut res: Result<()> = Ok(());
    let edit_last = Subs::Edit {
        script_name: Some("-".to_owned()),
        exact: false,
        executable: None,
        content: None,
    };
    let sub = root.subcmd.as_ref().unwrap_or(&edit_last);

    let tags = root.tags.clone();

    match sub {
        Subs::Other(cmds) => {
            log::info!("純執行模式");
            let run = Subs::Run {
                script_name: cmds[0].clone(),
                args: cmds[1..cmds.len()].iter().map(|s| s.clone()).collect(),
            };
            root.subcmd = Some(run);
            return main_inner(root, hs);
        }
        Subs::Edit {
            script_name,
            executable: ty,
            exact,
            content,
        } => {
            let final_ty: ScriptType;
            let script = if let Some(name) = script_name {
                if let Some(h) = get_info_mut(name, &mut hs, *exact)? {
                    if let Some(ty) = ty {
                        log::warn!("已存在的腳本無需再指定類型");
                        if ty != &h.ty {
                            return Err(Error::TypeMismatch {
                                expect: *ty,
                                actual: h.ty,
                            });
                        }
                    }
                    final_ty = h.ty;
                    log::debug!("打開既有命名腳本：{:?}", name);
                    path::open_script(&h.name, h.ty, true)
                        .context(format!("打開命名腳本失敗：{:?}", name))?
                } else {
                    final_ty = ty.unwrap_or_default();
                    log::debug!("打開新命名腳本：{:?}", name);
                    path::open_script(AsRef::<str>::as_ref(name), ty.unwrap_or_default(), false)
                        .context(format!("打開新命名腳本失敗：{:?}", name))?
                }
            } else {
                final_ty = ty.unwrap_or_default();
                log::debug!("打開新匿名腳本");
                path::open_new_anonymous(ty.unwrap_or_default()).context("打開新匿名腳本失敗")?
            };

            if let Some(content) = content {
                log::info!("快速編輯 {:?}", script.name);
                if script.path.exists() {
                    return Err(Error::PathExist(script.path));
                }
                util::fast_write_script(&script, content)?;
            } else {
                log::info!("編輯 {:?}", script.name);
                util::prepare_script(&script.path, final_ty)?;
                let cmd = util::create_cmd("vim", &[script.path]);
                let stat = util::run_cmd("vim", cmd)?;
                log::debug!("編輯器返回：{:?}", stat);
            }

            let name = script.name.into_static();
            let h = hs.entry(&name).or_insert(ScriptInfo::new(
                name,
                final_ty,
                tags.into_allowed_iter(),
            )?);
            h.edit_time = Utc::now();
        }
        Subs::Run { script_name, args } => {
            let h = get_info_mut_strict(script_name, &mut hs, false)?;
            log::info!("執行 {:?}", h.name);
            let script = path::open_script(&h.name, h.ty, true)?;
            match util::run(&script, &h, &args) {
                Err(e @ Error::ScriptError(_)) => res = Err(e),
                Err(e) => return Err(e),
                Ok(_) => (),
            }
            h.exec_time = Some(Utc::now());
        }
        Subs::Cat { script_name } => {
            let h = get_info_mut_strict(script_name, &mut hs, false)?;
            let script = path::open_script(&h.name, h.ty, true)?;
            log::info!("打印 {:?}", script.name);
            let content = util::read_file(&script.path)?;
            println!("{}", content);
        }
        Subs::LS(list) => {
            let opt = ListOptions {
                long: list.long,
                pattern: &list.pattern,
            };
            let stdout = std::io::stdout();
            fmt_list(&mut stdout.lock(), hs.into_iter(), &opt)?;
            return Ok(());
        }
        Subs::RM { scripts, exact } => {
            for script_name in scripts.into_iter() {
                let h = get_info_mut_strict(script_name, &mut hs, *exact)?;
                // TODO: 若是模糊搜出來的，問一下使用者是不是真的要刪
                let script = path::open_script(&h.name, h.ty, true)?;
                log::info!("刪除 {:?}", script);
                util::remove(&script)?;
                let name = script.name.into_static();
                hs.remove(&name);
            }
        }
        Subs::CP { exact, origin, new } => {
            let h = get_info_mut_strict(origin, &mut hs, *exact)?;
            let new_name = new.as_script_name()?;
            let og_script = path::open_script(&h.name, h.ty, true)?;
            let new_script = path::open_script(&new_name, h.ty, false)?;
            if new_script.path.exists() {
                return Err(Error::PathExist(new_script.path));
            }
            util::cp(&og_script, &new_script)?;
            let new_info = ScriptInfo {
                name: new_name.into_static(),
                edit_time: Utc::now(),
                ..h.clone()
            };
            hs.insert(new_info);
        }
        Subs::MV {
            exact,
            origin,
            new,
            tags,
            executable: ty,
        } => {
            let h = get_info_mut_strict(origin, &mut hs, *exact)?;
            let og_script = path::open_script(&h.name, h.ty, true)?;
            let new_ty = ty.unwrap_or(h.ty);
            let new_name = match new {
                Some(s) => s.as_script_name()?,
                None => h.name.clone(),
            };
            let new_script = path::open_script(&new_name, new_ty, false)?;
            util::mv(&og_script, &new_script)?;

            h.name = new_name.into_static();
            h.edit_time = Utc::now();
            h.ty = new_ty;
            if let Some(tags) = tags {
                h.tags = tags.clone().into_allowed_iter().collect();
            }
        }
        _ => unimplemented!(),
    }
    path::store_history(hs.into_iter_all())?;
    res
}
