#![cfg_attr(rustfmt, rustfmt_skip)]
type GlobalID = ID;
use supplement::core::*;

pub const ID_VAL_HS_HOME: id::SingleVal<GlobalID> = id::SingleVal::new(ID::ValHsHome);
const VAL_HS_HOME: Flag<GlobalID> = Flag {
    short: &['H'],
    long: &["hs-home"],
    description: "Path to hyper script home",
    once: true,
    ty: flag_type::Type::new_valued(ID_VAL_HS_HOME.into(), CompleteWithEqual::NoNeed, &[]),
};
pub const ID_VAL_NO_TRACE: id::NoVal = id::NoVal::new_certain(line!());
const VAL_NO_TRACE: Flag<GlobalID> = Flag {
    short: &[],
    long: &["no-trace"],
    description: "Don't record history",
    once: false,
    ty: flag_type::Type::new_bool(ID_VAL_NO_TRACE),
};
pub const ID_VAL_HUMBLE: id::NoVal = id::NoVal::new_certain(line!());
const VAL_HUMBLE: Flag<GlobalID> = Flag {
    short: &[],
    long: &["humble"],
    description: "Don't affect script time order (but still record history and affect time filter)",
    once: false,
    ty: flag_type::Type::new_bool(ID_VAL_HUMBLE),
};
pub const ID_VAL_ARCHAEOLOGY: id::NoVal = id::NoVal::new_certain(line!());
const VAL_ARCHAEOLOGY: Flag<GlobalID> = Flag {
    short: &['A'],
    long: &["archaeology"],
    description: "Show scripts NOT within recent days",
    once: false,
    ty: flag_type::Type::new_bool(ID_VAL_ARCHAEOLOGY),
};
pub const ID_VAL_NO_ALIAS: id::NoVal = id::NoVal::new_certain(line!());
const VAL_NO_ALIAS: Flag<GlobalID> = Flag {
    short: &[],
    long: &["no-alias"],
    description: "",
    once: true,
    ty: flag_type::Type::new_bool(ID_VAL_NO_ALIAS),
};
pub const ID_VAL_SELECT: id::MultiVal<GlobalID> = id::MultiVal::new(ID::ValSelect);
const VAL_SELECT: Flag<GlobalID> = Flag {
    short: &['s'],
    long: &["select"],
    description: "Select by tags, e.g. `all,^remove`",
    once: false,
    ty: flag_type::Type::new_valued(ID_VAL_SELECT.into(), CompleteWithEqual::NoNeed, &[]),
};
pub const ID_VAL_TOGGLE: id::MultiVal<GlobalID> = id::MultiVal::new(ID::ValToggle);
const VAL_TOGGLE: Flag<GlobalID> = Flag {
    short: &[],
    long: &["toggle"],
    description: "Toggle named selector temporarily",
    once: false,
    ty: flag_type::Type::new_valued(ID_VAL_TOGGLE.into(), CompleteWithEqual::NoNeed, &[]),
};
pub const ID_VAL_ALL: id::NoVal = id::NoVal::new_certain(line!());
const VAL_ALL: Flag<GlobalID> = Flag {
    short: &['a'],
    long: &["all"],
    description: "Shorthand for `-s=all,^remove --timeless`",
    once: false,
    ty: flag_type::Type::new_bool(ID_VAL_ALL),
};
pub const ID_VAL_RECENT: id::SingleVal<GlobalID> = id::SingleVal::new(ID::ValRecent);
const VAL_RECENT: Flag<GlobalID> = Flag {
    short: &[],
    long: &["recent"],
    description: "Show scripts within recent days.",
    once: false,
    ty: flag_type::Type::new_valued(ID_VAL_RECENT.into(), CompleteWithEqual::NoNeed, &[]),
};
pub const ID_VAL_TIMELESS: id::NoVal = id::NoVal::new_certain(line!());
const VAL_TIMELESS: Flag<GlobalID> = Flag {
    short: &[],
    long: &["timeless"],
    description: "Show scripts of all time.",
    once: false,
    ty: flag_type::Type::new_bool(ID_VAL_TIMELESS),
};
pub const ID_VAL_PROMPT_LEVEL: id::SingleVal<GlobalID> = id::SingleVal::new_certain(line!());
const VAL_PROMPT_LEVEL: Flag<GlobalID> = Flag {
    short: &[],
    long: &["prompt-level"],
    description: "Prompt level of fuzzy finder.",
    once: true,
    ty: flag_type::Type::new_valued(ID_VAL_PROMPT_LEVEL.into(), CompleteWithEqual::NoNeed, &[("always", ""), ("never", ""), ("smart", ""), ("on-multi-fuzz", "")]),
};
pub const ID_VAL_VERSION: id::NoVal = id::NoVal::new_certain(line!());
const VAL_VERSION: Flag<GlobalID> = Flag {
    short: &['V'],
    long: &["version"],
    description: "Print version",
    once: true,
    ty: flag_type::Type::new_bool(ID_VAL_VERSION),
};
pub const ID_EXTERNAL: id::MultiVal<GlobalID> = id::MultiVal::new(ID::External);
const EXTERNAL: Arg<GlobalID> = Arg {
    id: ID_EXTERNAL.into(),
    max_values: 18446744073709551615,
    possible_values: &[],
};
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ID {
    External,
    ValHsHome,
    ValSelect,
    ValToggle,
    ValRecent,
    CMDHelp(help::ID),
    CMDMigrate(migrate::ID),
    CMDEdit(edit::ID),
    CMDAlias(alias::ID),
    CMDConfig(config::ID),
    CMDRun(run::ID),
    CMDWhich(which::ID),
    CMDCat(cat::ID),
    CMDRm(rm::ID),
    CMDRecent(recent::ID),
    CMDLs(ls::ID),
    CMDTypes(types::ID),
    CMDCp(cp::ID),
    CMDMv(mv::ID),
    CMDTags(tags::ID),
    CMDHistory(history::ID),
    CMDTop(top::ID),
}
pub const CMD: Command<GlobalID> = Command {
    name: "hyper-scripter",
    description: "The script managing tool for script lovers",
    all_flags: &[VAL_HS_HOME, VAL_NO_TRACE, VAL_HUMBLE, VAL_ARCHAEOLOGY, VAL_NO_ALIAS, VAL_SELECT, VAL_TOGGLE, VAL_ALL, VAL_RECENT, VAL_TIMELESS, VAL_PROMPT_LEVEL, VAL_VERSION],
    args: &[EXTERNAL],
    commands: &[help::CMD, migrate::CMD, edit::CMD, alias::CMD, config::CMD, run::CMD, which::CMD, cat::CMD, rm::CMD, recent::CMD, ls::CMD, types::CMD, cp::CMD, mv::CMD, tags::CMD, history::CMD, top::CMD],
};
pub mod help {
    use super::GlobalID as GlobalID;
    use supplement::core::*;

    pub const ID_VAL_ARGS: id::MultiVal<GlobalID> = id::MultiVal::new(super::ID::CMDHelp(ID::ValArgs));
    const VAL_ARGS: Arg<GlobalID> = Arg {
        id: ID_VAL_ARGS.into(),
        max_values: 18446744073709551615,
        possible_values: &[],
    };
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum ID {
        ValArgs,
    }
    pub(super) const CMD: Command<GlobalID> = Command {
        name: "help",
        description: "Prints this message, the help of the given subcommand(s), or a script's help message.",
        all_flags: &[super::VAL_NO_TRACE, super::VAL_HUMBLE, super::VAL_ARCHAEOLOGY, super::VAL_SELECT, super::VAL_ALL, super::VAL_RECENT, super::VAL_TIMELESS],
        args: &[VAL_ARGS],
        commands: &[],
    };
}
pub mod migrate {
    use super::GlobalID as GlobalID;
    use supplement::core::*;

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum ID {
    }
    pub(super) const CMD: Command<GlobalID> = Command {
        name: "migrate",
        description: "Migrate the database",
        all_flags: &[super::VAL_NO_TRACE, super::VAL_HUMBLE, super::VAL_ARCHAEOLOGY, super::VAL_SELECT, super::VAL_ALL, super::VAL_RECENT, super::VAL_TIMELESS],
        args: &[],
        commands: &[],
    };
}
pub mod edit {
    use super::GlobalID as GlobalID;
    use supplement::core::*;

    pub const ID_VAL_TY: id::SingleVal<GlobalID> = id::SingleVal::new(super::ID::CMDEdit(ID::ValTy));
    const VAL_TY: Flag<GlobalID> = Flag {
        short: &['T'],
        long: &["ty"],
        description: "Type of the script, e.g. `sh`",
        once: true,
        ty: flag_type::Type::new_valued(ID_VAL_TY.into(), CompleteWithEqual::NoNeed, &[]),
    };
    pub const ID_VAL_NO_TEMPLATE: id::NoVal = id::NoVal::new_certain(line!());
    const VAL_NO_TEMPLATE: Flag<GlobalID> = Flag {
        short: &['n'],
        long: &["no-template"],
        description: "",
        once: true,
        ty: flag_type::Type::new_bool(ID_VAL_NO_TEMPLATE),
    };
    pub const ID_VAL_TAGS: id::SingleVal<GlobalID> = id::SingleVal::new(super::ID::CMDEdit(ID::ValTags));
    const VAL_TAGS: Flag<GlobalID> = Flag {
        short: &['t'],
        long: &["tags"],
        description: "Tags of the script",
        once: true,
        ty: flag_type::Type::new_valued(ID_VAL_TAGS.into(), CompleteWithEqual::NoNeed, &[]),
    };
    pub const ID_VAL_FAST: id::NoVal = id::NoVal::new_certain(line!());
    const VAL_FAST: Flag<GlobalID> = Flag {
        short: &['f'],
        long: &["fast"],
        description: "Create script without invoking the editor",
        once: true,
        ty: flag_type::Type::new_bool(ID_VAL_FAST),
    };
    pub const ID_VAL_EDIT_QUERY: id::MultiVal<GlobalID> = id::MultiVal::new(super::ID::CMDEdit(ID::ValEditQuery));
    const VAL_EDIT_QUERY: Arg<GlobalID> = Arg {
        id: ID_VAL_EDIT_QUERY.into(),
        max_values: 18446744073709551615,
        possible_values: &[],
    };
    pub const ID_VAL_CONTENT: id::MultiVal<GlobalID> = id::MultiVal::new(super::ID::CMDEdit(ID::ValContent));
    const VAL_CONTENT: Arg<GlobalID> = Arg {
        id: ID_VAL_CONTENT.into(),
        max_values: 18446744073709551615,
        possible_values: &[],
    };
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum ID {
        ValEditQuery,
        ValContent,
        ValTy,
        ValTags,
    }
    pub(super) const CMD: Command<GlobalID> = Command {
        name: "edit",
        description: "Edit hyper script",
        all_flags: &[VAL_TY, VAL_NO_TEMPLATE, VAL_TAGS, VAL_FAST, super::VAL_NO_TRACE, super::VAL_HUMBLE, super::VAL_ARCHAEOLOGY, super::VAL_SELECT, super::VAL_ALL, super::VAL_RECENT, super::VAL_TIMELESS],
        args: &[VAL_EDIT_QUERY, VAL_CONTENT],
        commands: &[],
    };
}
pub mod alias {
    use super::GlobalID as GlobalID;
    use supplement::core::*;

    pub const ID_VAL_UNSET: id::NoVal = id::NoVal::new_certain(line!());
    const VAL_UNSET: Flag<GlobalID> = Flag {
        short: &['u'],
        long: &["unset"],
        description: "Unset an alias.",
        once: true,
        ty: flag_type::Type::new_bool(ID_VAL_UNSET),
    };
    pub const ID_VAL_BEFORE: id::SingleVal<GlobalID> = id::SingleVal::new(super::ID::CMDAlias(ID::ValBefore));
    const VAL_BEFORE: Arg<GlobalID> = Arg {
        id: ID_VAL_BEFORE.into(),
        max_values: 1,
        possible_values: &[],
    };
    pub const ID_VAL_AFTER: id::MultiVal<GlobalID> = id::MultiVal::new(super::ID::CMDAlias(ID::ValAfter));
    const VAL_AFTER: Arg<GlobalID> = Arg {
        id: ID_VAL_AFTER.into(),
        max_values: 18446744073709551615,
        possible_values: &[],
    };
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum ID {
        ValBefore,
        ValAfter,
    }
    pub(super) const CMD: Command<GlobalID> = Command {
        name: "alias",
        description: "Manage alias",
        all_flags: &[VAL_UNSET, super::VAL_NO_TRACE, super::VAL_HUMBLE, super::VAL_ARCHAEOLOGY, super::VAL_SELECT, super::VAL_ALL, super::VAL_RECENT, super::VAL_TIMELESS],
        args: &[VAL_BEFORE, VAL_AFTER],
        commands: &[],
    };
}
pub mod config {
    use super::GlobalID as GlobalID;
    use supplement::core::*;

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum ID {
    }
    pub(super) const CMD: Command<GlobalID> = Command {
        name: "config",
        description: "Print the path to script",
        all_flags: &[super::VAL_NO_TRACE, super::VAL_HUMBLE, super::VAL_ARCHAEOLOGY, super::VAL_SELECT, super::VAL_ALL, super::VAL_RECENT, super::VAL_TIMELESS],
        args: &[],
        commands: &[],
    };
}
pub mod run {
    use super::GlobalID as GlobalID;
    use supplement::core::*;

    pub const ID_VAL_NO_CAUTION: id::NoVal = id::NoVal::new_certain(line!());
    const VAL_NO_CAUTION: Flag<GlobalID> = Flag {
        short: &[],
        long: &["no-caution"],
        description: "Run caution scripts without warning",
        once: true,
        ty: flag_type::Type::new_bool(ID_VAL_NO_CAUTION),
    };
    pub const ID_VAL_DUMMY: id::NoVal = id::NoVal::new_certain(line!());
    const VAL_DUMMY: Flag<GlobalID> = Flag {
        short: &[],
        long: &["dummy"],
        description: "Add a dummy run history instead of actually running it",
        once: true,
        ty: flag_type::Type::new_bool(ID_VAL_DUMMY),
    };
    pub const ID_VAL_REPEAT: id::SingleVal<GlobalID> = id::SingleVal::new(super::ID::CMDRun(ID::ValRepeat));
    const VAL_REPEAT: Flag<GlobalID> = Flag {
        short: &['r'],
        long: &["repeat"],
        description: "",
        once: true,
        ty: flag_type::Type::new_valued(ID_VAL_REPEAT.into(), CompleteWithEqual::NoNeed, &[]),
    };
    pub const ID_VAL_PREVIOUS: id::SingleVal<GlobalID> = id::SingleVal::new_certain(line!());
    const VAL_PREVIOUS: Flag<GlobalID> = Flag {
        short: &['p'],
        long: &["previous"],
        description: "Use arguments from last run",
        once: true,
        ty: flag_type::Type::new_valued(ID_VAL_PREVIOUS.into(), CompleteWithEqual::Optional, &[("env", ""), ("args", ""), ("all", "")]),
    };
    pub const ID_VAL_ERROR_NO_PREVIOUS: id::NoVal = id::NoVal::new_certain(line!());
    const VAL_ERROR_NO_PREVIOUS: Flag<GlobalID> = Flag {
        short: &['E'],
        long: &["error-no-previous"],
        description: "Raise an error if --previous is given but there is no previous run",
        once: true,
        ty: flag_type::Type::new_bool(ID_VAL_ERROR_NO_PREVIOUS),
    };
    pub const ID_VAL_DIR: id::SingleVal<GlobalID> = id::SingleVal::new(super::ID::CMDRun(ID::ValDir));
    const VAL_DIR: Flag<GlobalID> = Flag {
        short: &['d'],
        long: &["dir"],
        description: "",
        once: true,
        ty: flag_type::Type::new_valued(ID_VAL_DIR.into(), CompleteWithEqual::NoNeed, &[]),
    };
    pub const ID_VAL_SCRIPT_QUERY: id::SingleVal<GlobalID> = id::SingleVal::new(super::ID::CMDRun(ID::ValScriptQuery));
    const VAL_SCRIPT_QUERY: Arg<GlobalID> = Arg {
        id: ID_VAL_SCRIPT_QUERY.into(),
        max_values: 1,
        possible_values: &[],
    };
    pub const ID_VAL_ARGS: id::MultiVal<GlobalID> = id::MultiVal::new(super::ID::CMDRun(ID::ValArgs));
    const VAL_ARGS: Arg<GlobalID> = Arg {
        id: ID_VAL_ARGS.into(),
        max_values: 18446744073709551615,
        possible_values: &[],
    };
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum ID {
        ValScriptQuery,
        ValArgs,
        ValRepeat,
        ValDir,
    }
    pub(super) const CMD: Command<GlobalID> = Command {
        name: "run",
        description: "Run the script",
        all_flags: &[VAL_NO_CAUTION, VAL_DUMMY, VAL_REPEAT, VAL_PREVIOUS, VAL_ERROR_NO_PREVIOUS, VAL_DIR, super::VAL_NO_TRACE, super::VAL_HUMBLE, super::VAL_ARCHAEOLOGY, super::VAL_SELECT, super::VAL_ALL, super::VAL_RECENT, super::VAL_TIMELESS],
        args: &[VAL_SCRIPT_QUERY, VAL_ARGS],
        commands: &[],
    };
}
pub mod which {
    use super::GlobalID as GlobalID;
    use supplement::core::*;

    pub const ID_VAL_QUERIES: id::MultiVal<GlobalID> = id::MultiVal::new(super::ID::CMDWhich(ID::ValQueries));
    const VAL_QUERIES: Arg<GlobalID> = Arg {
        id: ID_VAL_QUERIES.into(),
        max_values: 18446744073709551615,
        possible_values: &[],
    };
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum ID {
        ValQueries,
    }
    pub(super) const CMD: Command<GlobalID> = Command {
        name: "which",
        description: "Execute the script query and get the exact file",
        all_flags: &[super::VAL_NO_TRACE, super::VAL_HUMBLE, super::VAL_ARCHAEOLOGY, super::VAL_SELECT, super::VAL_ALL, super::VAL_RECENT, super::VAL_TIMELESS],
        args: &[VAL_QUERIES],
        commands: &[],
    };
}
pub mod cat {
    use super::GlobalID as GlobalID;
    use supplement::core::*;

    pub const ID_VAL_WITH: id::SingleVal<GlobalID> = id::SingleVal::new(super::ID::CMDCat(ID::ValWith));
    const VAL_WITH: Flag<GlobalID> = Flag {
        short: &[],
        long: &["with"],
        description: "Read with other program, e.g. bat",
        once: true,
        ty: flag_type::Type::new_valued(ID_VAL_WITH.into(), CompleteWithEqual::NoNeed, &[]),
    };
    pub const ID_VAL_QUERIES: id::MultiVal<GlobalID> = id::MultiVal::new(super::ID::CMDCat(ID::ValQueries));
    const VAL_QUERIES: Arg<GlobalID> = Arg {
        id: ID_VAL_QUERIES.into(),
        max_values: 18446744073709551615,
        possible_values: &[],
    };
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum ID {
        ValQueries,
        ValWith,
    }
    pub(super) const CMD: Command<GlobalID> = Command {
        name: "cat",
        description: "Print the script to standard output",
        all_flags: &[VAL_WITH, super::VAL_NO_TRACE, super::VAL_HUMBLE, super::VAL_ARCHAEOLOGY, super::VAL_SELECT, super::VAL_ALL, super::VAL_RECENT, super::VAL_TIMELESS],
        args: &[VAL_QUERIES],
        commands: &[],
    };
}
pub mod rm {
    use super::GlobalID as GlobalID;
    use supplement::core::*;

    pub const ID_VAL_PURGE: id::NoVal = id::NoVal::new_certain(line!());
    const VAL_PURGE: Flag<GlobalID> = Flag {
        short: &[],
        long: &["purge"],
        description: "Actually remove scripts, rather than hiding them with tag.",
        once: true,
        ty: flag_type::Type::new_bool(ID_VAL_PURGE),
    };
    pub const ID_VAL_QUERIES: id::MultiVal<GlobalID> = id::MultiVal::new(super::ID::CMDRm(ID::ValQueries));
    const VAL_QUERIES: Arg<GlobalID> = Arg {
        id: ID_VAL_QUERIES.into(),
        max_values: 18446744073709551615,
        possible_values: &[],
    };
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum ID {
        ValQueries,
    }
    pub(super) const CMD: Command<GlobalID> = Command {
        name: "rm",
        description: "Remove the script",
        all_flags: &[VAL_PURGE, super::VAL_NO_TRACE, super::VAL_HUMBLE, super::VAL_ARCHAEOLOGY, super::VAL_SELECT, super::VAL_ALL, super::VAL_RECENT, super::VAL_TIMELESS],
        args: &[VAL_QUERIES],
        commands: &[],
    };
}
pub mod recent {
    use super::GlobalID as GlobalID;
    use supplement::core::*;

    pub const ID_VAL_RECENT_FILTER: id::SingleVal<GlobalID> = id::SingleVal::new(super::ID::CMDRecent(ID::ValRecentFilter));
    const VAL_RECENT_FILTER: Arg<GlobalID> = Arg {
        id: ID_VAL_RECENT_FILTER.into(),
        max_values: 1,
        possible_values: &[],
    };
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum ID {
        ValRecentFilter,
    }
    pub(super) const CMD: Command<GlobalID> = Command {
        name: "recent",
        description: "Set recent filter",
        all_flags: &[super::VAL_NO_TRACE, super::VAL_HUMBLE, super::VAL_ARCHAEOLOGY, super::VAL_SELECT, super::VAL_ALL, super::VAL_RECENT, super::VAL_TIMELESS],
        args: &[VAL_RECENT_FILTER],
        commands: &[],
    };
}
pub mod ls {
    use super::GlobalID as GlobalID;
    use supplement::core::*;

    pub const ID_VAL_LONG: id::NoVal = id::NoVal::new_certain(line!());
    const VAL_LONG: Flag<GlobalID> = Flag {
        short: &['l'],
        long: &["long"],
        description: "Show verbose information.",
        once: true,
        ty: flag_type::Type::new_bool(ID_VAL_LONG),
    };
    pub const ID_VAL_GROUPING: id::SingleVal<GlobalID> = id::SingleVal::new_certain(line!());
    const VAL_GROUPING: Flag<GlobalID> = Flag {
        short: &[],
        long: &["grouping"],
        description: "Grouping style.",
        once: true,
        ty: flag_type::Type::new_valued(ID_VAL_GROUPING.into(), CompleteWithEqual::NoNeed, &[("tag", ""), ("tree", ""), ("none", "")]),
    };
    pub const ID_VAL_LIMIT: id::SingleVal<GlobalID> = id::SingleVal::new(super::ID::CMDLs(ID::ValLimit));
    const VAL_LIMIT: Flag<GlobalID> = Flag {
        short: &[],
        long: &["limit"],
        description: "Limit the amount of scripts found.",
        once: true,
        ty: flag_type::Type::new_valued(ID_VAL_LIMIT.into(), CompleteWithEqual::NoNeed, &[]),
    };
    pub const ID_VAL_PLAIN: id::NoVal = id::NoVal::new_certain(line!());
    const VAL_PLAIN: Flag<GlobalID> = Flag {
        short: &[],
        long: &["plain"],
        description: "No color and other decoration.",
        once: true,
        ty: flag_type::Type::new_bool(ID_VAL_PLAIN),
    };
    pub const ID_VAL_FORMAT: id::SingleVal<GlobalID> = id::SingleVal::new(super::ID::CMDLs(ID::ValFormat));
    const VAL_FORMAT: Flag<GlobalID> = Flag {
        short: &[],
        long: &["format"],
        description: "Define the formatting for each script.",
        once: true,
        ty: flag_type::Type::new_valued(ID_VAL_FORMAT.into(), CompleteWithEqual::NoNeed, &[]),
    };
    pub const ID_VAL_QUERIES: id::MultiVal<GlobalID> = id::MultiVal::new(super::ID::CMDLs(ID::ValQueries));
    const VAL_QUERIES: Arg<GlobalID> = Arg {
        id: ID_VAL_QUERIES.into(),
        max_values: 18446744073709551615,
        possible_values: &[],
    };
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum ID {
        ValQueries,
        ValLimit,
        ValFormat,
    }
    pub(super) const CMD: Command<GlobalID> = Command {
        name: "ls",
        description: "List hyper scripts",
        all_flags: &[VAL_LONG, VAL_GROUPING, VAL_LIMIT, VAL_PLAIN, VAL_FORMAT, super::VAL_NO_TRACE, super::VAL_HUMBLE, super::VAL_ARCHAEOLOGY, super::VAL_SELECT, super::VAL_ALL, super::VAL_RECENT, super::VAL_TIMELESS],
        args: &[VAL_QUERIES],
        commands: &[],
    };
}
pub mod types {
    use super::GlobalID as GlobalID;
    use supplement::core::*;

    pub const ID_VAL_EDIT: id::NoVal = id::NoVal::new_certain(line!());
    const VAL_EDIT: Flag<GlobalID> = Flag {
        short: &['e'],
        long: &["edit"],
        description: "",
        once: true,
        ty: flag_type::Type::new_bool(ID_VAL_EDIT),
    };
    pub const ID_VAL_TY: id::SingleVal<GlobalID> = id::SingleVal::new(super::ID::CMDTypes(ID::ValTy));
    const VAL_TY: Arg<GlobalID> = Arg {
        id: ID_VAL_TY.into(),
        max_values: 1,
        possible_values: &[],
    };
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum ID {
        ValTy,
    }
    pub(super) const CMD: Command<GlobalID> = Command {
        name: "types",
        description: "Manage script types",
        all_flags: &[VAL_EDIT, super::VAL_NO_TRACE, super::VAL_HUMBLE, super::VAL_ARCHAEOLOGY, super::VAL_SELECT, super::VAL_ALL, super::VAL_RECENT, super::VAL_TIMELESS],
        args: &[VAL_TY],
        commands: &[],
    };
}
pub mod cp {
    use super::GlobalID as GlobalID;
    use supplement::core::*;

    pub const ID_VAL_TAGS: id::SingleVal<GlobalID> = id::SingleVal::new(super::ID::CMDCp(ID::ValTags));
    const VAL_TAGS: Flag<GlobalID> = Flag {
        short: &['t'],
        long: &["tags"],
        description: "Tags of the script",
        once: true,
        ty: flag_type::Type::new_valued(ID_VAL_TAGS.into(), CompleteWithEqual::NoNeed, &[]),
    };
    pub const ID_VAL_ORIGIN: id::SingleVal<GlobalID> = id::SingleVal::new(super::ID::CMDCp(ID::ValOrigin));
    const VAL_ORIGIN: Arg<GlobalID> = Arg {
        id: ID_VAL_ORIGIN.into(),
        max_values: 1,
        possible_values: &[],
    };
    pub const ID_VAL_NEW: id::SingleVal<GlobalID> = id::SingleVal::new(super::ID::CMDCp(ID::ValNew));
    const VAL_NEW: Arg<GlobalID> = Arg {
        id: ID_VAL_NEW.into(),
        max_values: 1,
        possible_values: &[],
    };
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum ID {
        ValOrigin,
        ValNew,
        ValTags,
    }
    pub(super) const CMD: Command<GlobalID> = Command {
        name: "cp",
        description: "Copy the script to another one",
        all_flags: &[VAL_TAGS, super::VAL_NO_TRACE, super::VAL_HUMBLE, super::VAL_ARCHAEOLOGY, super::VAL_SELECT, super::VAL_ALL, super::VAL_RECENT, super::VAL_TIMELESS],
        args: &[VAL_ORIGIN, VAL_NEW],
        commands: &[],
    };
}
pub mod mv {
    use super::GlobalID as GlobalID;
    use supplement::core::*;

    pub const ID_VAL_TY: id::SingleVal<GlobalID> = id::SingleVal::new(super::ID::CMDMv(ID::ValTy));
    const VAL_TY: Flag<GlobalID> = Flag {
        short: &['T'],
        long: &["ty"],
        description: "Type of the script, e.g. `sh`",
        once: true,
        ty: flag_type::Type::new_valued(ID_VAL_TY.into(), CompleteWithEqual::NoNeed, &[]),
    };
    pub const ID_VAL_TAGS: id::SingleVal<GlobalID> = id::SingleVal::new(super::ID::CMDMv(ID::ValTags));
    const VAL_TAGS: Flag<GlobalID> = Flag {
        short: &['t'],
        long: &["tags"],
        description: "Tags of the script",
        once: true,
        ty: flag_type::Type::new_valued(ID_VAL_TAGS.into(), CompleteWithEqual::NoNeed, &[]),
    };
    pub const ID_VAL_ORIGIN: id::SingleVal<GlobalID> = id::SingleVal::new(super::ID::CMDMv(ID::ValOrigin));
    const VAL_ORIGIN: Arg<GlobalID> = Arg {
        id: ID_VAL_ORIGIN.into(),
        max_values: 1,
        possible_values: &[],
    };
    pub const ID_VAL_NEW: id::SingleVal<GlobalID> = id::SingleVal::new(super::ID::CMDMv(ID::ValNew));
    const VAL_NEW: Arg<GlobalID> = Arg {
        id: ID_VAL_NEW.into(),
        max_values: 1,
        possible_values: &[],
    };
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum ID {
        ValOrigin,
        ValNew,
        ValTy,
        ValTags,
    }
    pub(super) const CMD: Command<GlobalID> = Command {
        name: "mv",
        description: "Move the script to another one",
        all_flags: &[VAL_TY, VAL_TAGS, super::VAL_NO_TRACE, super::VAL_HUMBLE, super::VAL_ARCHAEOLOGY, super::VAL_SELECT, super::VAL_ALL, super::VAL_RECENT, super::VAL_TIMELESS],
        args: &[VAL_ORIGIN, VAL_NEW],
        commands: &[],
    };
}
pub mod tags {
    use super::GlobalID as GlobalID;
    use supplement::core::*;

    pub const ID_EXTERNAL: id::MultiVal<GlobalID> = id::MultiVal::new(super::ID::CMDTags(ID::External));
    const EXTERNAL: Arg<GlobalID> = Arg {
        id: ID_EXTERNAL.into(),
        max_values: 18446744073709551615,
        possible_values: &[],
    };
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum ID {
        External,
        CMDUnset(unset::ID),
        CMDSet(set::ID),
        CMDToggle(toggle::ID),
    }
    pub(super) const CMD: Command<GlobalID> = Command {
        name: "tags",
        description: "Manage script tags",
        all_flags: &[super::VAL_NO_TRACE, super::VAL_HUMBLE, super::VAL_ARCHAEOLOGY, super::VAL_SELECT, super::VAL_ALL, super::VAL_RECENT, super::VAL_TIMELESS],
        args: &[EXTERNAL],
        commands: &[unset::CMD, set::CMD, toggle::CMD],
    };
    pub mod unset {
        use super::super::GlobalID as GlobalID;
        use supplement::core::*;

        pub const ID_VAL_NAME: id::SingleVal<GlobalID> = id::SingleVal::new(super::super::ID::CMDTags(super::ID::CMDUnset(ID::ValName)));
        const VAL_NAME: Arg<GlobalID> = Arg {
            id: ID_VAL_NAME.into(),
            max_values: 1,
            possible_values: &[],
        };
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum ID {
            ValName,
        }
        pub(super) const CMD: Command<GlobalID> = Command {
            name: "unset",
            description: "",
            all_flags: &[super::super::VAL_NO_TRACE, super::super::VAL_HUMBLE, super::super::VAL_ARCHAEOLOGY, super::super::VAL_SELECT, super::super::VAL_ALL, super::super::VAL_RECENT, super::super::VAL_TIMELESS],
            args: &[VAL_NAME],
            commands: &[],
        };
    }
    pub mod set {
        use super::super::GlobalID as GlobalID;
        use supplement::core::*;

        pub const ID_VAL_NAME: id::SingleVal<GlobalID> = id::SingleVal::new(super::super::ID::CMDTags(super::ID::CMDSet(ID::ValName)));
        const VAL_NAME: Flag<GlobalID> = Flag {
            short: &['n'],
            long: &["name"],
            description: "",
            once: true,
            ty: flag_type::Type::new_valued(ID_VAL_NAME.into(), CompleteWithEqual::NoNeed, &[]),
        };
        pub const ID_VAL_CONTENT: id::SingleVal<GlobalID> = id::SingleVal::new(super::super::ID::CMDTags(super::ID::CMDSet(ID::ValContent)));
        const VAL_CONTENT: Arg<GlobalID> = Arg {
            id: ID_VAL_CONTENT.into(),
            max_values: 1,
            possible_values: &[],
        };
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum ID {
            ValContent,
            ValName,
        }
        pub(super) const CMD: Command<GlobalID> = Command {
            name: "set",
            description: "",
            all_flags: &[VAL_NAME, super::super::VAL_NO_TRACE, super::super::VAL_HUMBLE, super::super::VAL_ARCHAEOLOGY, super::super::VAL_SELECT, super::super::VAL_ALL, super::super::VAL_RECENT, super::super::VAL_TIMELESS],
            args: &[VAL_CONTENT],
            commands: &[],
        };
    }
    pub mod toggle {
        use super::super::GlobalID as GlobalID;
        use supplement::core::*;

        pub const ID_VAL_NAMES: id::MultiVal<GlobalID> = id::MultiVal::new(super::super::ID::CMDTags(super::ID::CMDToggle(ID::ValNames)));
        const VAL_NAMES: Arg<GlobalID> = Arg {
            id: ID_VAL_NAMES.into(),
            max_values: 18446744073709551615,
            possible_values: &[],
        };
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum ID {
            ValNames,
        }
        pub(super) const CMD: Command<GlobalID> = Command {
            name: "toggle",
            description: "",
            all_flags: &[super::super::VAL_NO_TRACE, super::super::VAL_HUMBLE, super::super::VAL_ARCHAEOLOGY, super::super::VAL_SELECT, super::super::VAL_ALL, super::super::VAL_RECENT, super::super::VAL_TIMELESS],
            args: &[VAL_NAMES],
            commands: &[],
        };
    }
}
pub mod history {
    use super::GlobalID as GlobalID;
    use supplement::core::*;

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum ID {
        CMDRm(rm::ID),
        CMDRmId(rm_id::ID),
        CMDHumble(humble::ID),
        CMDShow(show::ID),
        CMDNeglect(neglect::ID),
        CMDAmend(amend::ID),
        CMDTidy(tidy::ID),
    }
    pub(super) const CMD: Command<GlobalID> = Command {
        name: "history",
        description: "Manage script history",
        all_flags: &[super::VAL_NO_TRACE, super::VAL_HUMBLE, super::VAL_ARCHAEOLOGY, super::VAL_SELECT, super::VAL_ALL, super::VAL_RECENT, super::VAL_TIMELESS],
        args: &[],
        commands: &[rm::CMD, rm_id::CMD, humble::CMD, show::CMD, neglect::CMD, amend::CMD, tidy::CMD],
    };
    pub mod rm {
        use super::super::GlobalID as GlobalID;
        use supplement::core::*;

        pub const ID_VAL_DIR: id::SingleVal<GlobalID> = id::SingleVal::new(super::super::ID::CMDHistory(super::ID::CMDRm(ID::ValDir)));
        const VAL_DIR: Flag<GlobalID> = Flag {
            short: &['d'],
            long: &["dir"],
            description: "",
            once: true,
            ty: flag_type::Type::new_valued(ID_VAL_DIR.into(), CompleteWithEqual::NoNeed, &[]),
        };
        pub const ID_VAL_DISPLAY: id::SingleVal<GlobalID> = id::SingleVal::new_certain(line!());
        const VAL_DISPLAY: Flag<GlobalID> = Flag {
            short: &[],
            long: &["display"],
            description: "",
            once: true,
            ty: flag_type::Type::new_valued(ID_VAL_DISPLAY.into(), CompleteWithEqual::NoNeed, &[("env", ""), ("args", ""), ("all", "")]),
        };
        pub const ID_VAL_NO_HUMBLE: id::NoVal = id::NoVal::new_certain(line!());
        const VAL_NO_HUMBLE: Flag<GlobalID> = Flag {
            short: &[],
            long: &["no-humble"],
            description: "",
            once: true,
            ty: flag_type::Type::new_bool(ID_VAL_NO_HUMBLE),
        };
        pub const ID_VAL_QUERIES: id::MultiVal<GlobalID> = id::MultiVal::new(super::super::ID::CMDHistory(super::ID::CMDRm(ID::ValQueries)));
        const VAL_QUERIES: Arg<GlobalID> = Arg {
            id: ID_VAL_QUERIES.into(),
            max_values: 18446744073709551615,
            possible_values: &[],
        };
        pub const ID_VAL_RANGE: id::SingleVal<GlobalID> = id::SingleVal::new(super::super::ID::CMDHistory(super::ID::CMDRm(ID::ValRange)));
        const VAL_RANGE: Arg<GlobalID> = Arg {
            id: ID_VAL_RANGE.into(),
            max_values: 1,
            possible_values: &[],
        };
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum ID {
            ValQueries,
            ValRange,
            ValDir,
        }
        pub(super) const CMD: Command<GlobalID> = Command {
            name: "rm",
            description: "",
            all_flags: &[VAL_DIR, VAL_DISPLAY, VAL_NO_HUMBLE, super::super::VAL_NO_TRACE, super::super::VAL_HUMBLE, super::super::VAL_ARCHAEOLOGY, super::super::VAL_SELECT, super::super::VAL_ALL, super::super::VAL_RECENT, super::super::VAL_TIMELESS],
            args: &[VAL_QUERIES, VAL_RANGE],
            commands: &[],
        };
    }
    pub mod rm_id {
        use super::super::GlobalID as GlobalID;
        use supplement::core::*;

        pub const ID_VAL_EVENT_ID: id::SingleVal<GlobalID> = id::SingleVal::new(super::super::ID::CMDHistory(super::ID::CMDRmId(ID::ValEventId)));
        const VAL_EVENT_ID: Arg<GlobalID> = Arg {
            id: ID_VAL_EVENT_ID.into(),
            max_values: 1,
            possible_values: &[],
        };
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum ID {
            ValEventId,
        }
        pub(super) const CMD: Command<GlobalID> = Command {
            name: "rm-id",
            description: "Remove an event by it's id. Useful if you want to keep those illegal arguments from polluting the history.",
            all_flags: &[super::super::VAL_NO_TRACE, super::super::VAL_HUMBLE, super::super::VAL_ARCHAEOLOGY, super::super::VAL_SELECT, super::super::VAL_ALL, super::super::VAL_RECENT, super::super::VAL_TIMELESS],
            args: &[VAL_EVENT_ID],
            commands: &[],
        };
    }
    pub mod humble {
        use super::super::GlobalID as GlobalID;
        use supplement::core::*;

        pub const ID_VAL_EVENT_ID: id::SingleVal<GlobalID> = id::SingleVal::new(super::super::ID::CMDHistory(super::ID::CMDHumble(ID::ValEventId)));
        const VAL_EVENT_ID: Arg<GlobalID> = Arg {
            id: ID_VAL_EVENT_ID.into(),
            max_values: 1,
            possible_values: &[],
        };
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum ID {
            ValEventId,
        }
        pub(super) const CMD: Command<GlobalID> = Command {
            name: "humble",
            description: "Humble an event by it's id",
            all_flags: &[super::super::VAL_NO_TRACE, super::super::VAL_HUMBLE, super::super::VAL_ARCHAEOLOGY, super::super::VAL_SELECT, super::super::VAL_ALL, super::super::VAL_RECENT, super::super::VAL_TIMELESS],
            args: &[VAL_EVENT_ID],
            commands: &[],
        };
    }
    pub mod show {
        use super::super::GlobalID as GlobalID;
        use supplement::core::*;

        pub const ID_VAL_LIMIT: id::SingleVal<GlobalID> = id::SingleVal::new(super::super::ID::CMDHistory(super::ID::CMDShow(ID::ValLimit)));
        const VAL_LIMIT: Flag<GlobalID> = Flag {
            short: &['l'],
            long: &["limit"],
            description: "",
            once: true,
            ty: flag_type::Type::new_valued(ID_VAL_LIMIT.into(), CompleteWithEqual::NoNeed, &[]),
        };
        pub const ID_VAL_WITH_NAME: id::NoVal = id::NoVal::new_certain(line!());
        const VAL_WITH_NAME: Flag<GlobalID> = Flag {
            short: &[],
            long: &["with-name"],
            description: "",
            once: true,
            ty: flag_type::Type::new_bool(ID_VAL_WITH_NAME),
        };
        pub const ID_VAL_NO_HUMBLE: id::NoVal = id::NoVal::new_certain(line!());
        const VAL_NO_HUMBLE: Flag<GlobalID> = Flag {
            short: &[],
            long: &["no-humble"],
            description: "",
            once: true,
            ty: flag_type::Type::new_bool(ID_VAL_NO_HUMBLE),
        };
        pub const ID_VAL_OFFSET: id::SingleVal<GlobalID> = id::SingleVal::new(super::super::ID::CMDHistory(super::ID::CMDShow(ID::ValOffset)));
        const VAL_OFFSET: Flag<GlobalID> = Flag {
            short: &['o'],
            long: &["offset"],
            description: "",
            once: true,
            ty: flag_type::Type::new_valued(ID_VAL_OFFSET.into(), CompleteWithEqual::NoNeed, &[]),
        };
        pub const ID_VAL_DIR: id::SingleVal<GlobalID> = id::SingleVal::new(super::super::ID::CMDHistory(super::ID::CMDShow(ID::ValDir)));
        const VAL_DIR: Flag<GlobalID> = Flag {
            short: &['d'],
            long: &["dir"],
            description: "",
            once: true,
            ty: flag_type::Type::new_valued(ID_VAL_DIR.into(), CompleteWithEqual::NoNeed, &[]),
        };
        pub const ID_VAL_DISPLAY: id::SingleVal<GlobalID> = id::SingleVal::new_certain(line!());
        const VAL_DISPLAY: Flag<GlobalID> = Flag {
            short: &[],
            long: &["display"],
            description: "",
            once: true,
            ty: flag_type::Type::new_valued(ID_VAL_DISPLAY.into(), CompleteWithEqual::NoNeed, &[("env", ""), ("args", ""), ("all", "")]),
        };
        pub const ID_VAL_QUERIES: id::MultiVal<GlobalID> = id::MultiVal::new(super::super::ID::CMDHistory(super::ID::CMDShow(ID::ValQueries)));
        const VAL_QUERIES: Arg<GlobalID> = Arg {
            id: ID_VAL_QUERIES.into(),
            max_values: 18446744073709551615,
            possible_values: &[],
        };
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum ID {
            ValQueries,
            ValLimit,
            ValOffset,
            ValDir,
        }
        pub(super) const CMD: Command<GlobalID> = Command {
            name: "show",
            description: "",
            all_flags: &[VAL_LIMIT, VAL_WITH_NAME, VAL_NO_HUMBLE, VAL_OFFSET, VAL_DIR, VAL_DISPLAY, super::super::VAL_NO_TRACE, super::super::VAL_HUMBLE, super::super::VAL_ARCHAEOLOGY, super::super::VAL_SELECT, super::super::VAL_ALL, super::super::VAL_RECENT, super::super::VAL_TIMELESS],
            args: &[VAL_QUERIES],
            commands: &[],
        };
    }
    pub mod neglect {
        use super::super::GlobalID as GlobalID;
        use supplement::core::*;

        pub const ID_VAL_QUERIES: id::MultiVal<GlobalID> = id::MultiVal::new(super::super::ID::CMDHistory(super::ID::CMDNeglect(ID::ValQueries)));
        const VAL_QUERIES: Arg<GlobalID> = Arg {
            id: ID_VAL_QUERIES.into(),
            max_values: 18446744073709551615,
            possible_values: &[],
        };
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum ID {
            ValQueries,
        }
        pub(super) const CMD: Command<GlobalID> = Command {
            name: "neglect",
            description: "",
            all_flags: &[super::super::VAL_NO_TRACE, super::super::VAL_HUMBLE, super::super::VAL_ARCHAEOLOGY, super::super::VAL_SELECT, super::super::VAL_ALL, super::super::VAL_RECENT, super::super::VAL_TIMELESS],
            args: &[VAL_QUERIES],
            commands: &[],
        };
    }
    pub mod amend {
        use super::super::GlobalID as GlobalID;
        use supplement::core::*;

        pub const ID_VAL_ENV: id::MultiVal<GlobalID> = id::MultiVal::new(super::super::ID::CMDHistory(super::ID::CMDAmend(ID::ValEnv)));
        const VAL_ENV: Flag<GlobalID> = Flag {
            short: &['e'],
            long: &["env"],
            description: "",
            once: false,
            ty: flag_type::Type::new_valued(ID_VAL_ENV.into(), CompleteWithEqual::NoNeed, &[]),
        };
        pub const ID_VAL_NO_ENV: id::NoVal = id::NoVal::new_certain(line!());
        const VAL_NO_ENV: Flag<GlobalID> = Flag {
            short: &[],
            long: &["no-env"],
            description: "",
            once: true,
            ty: flag_type::Type::new_bool(ID_VAL_NO_ENV),
        };
        pub const ID_VAL_EVENT_ID: id::SingleVal<GlobalID> = id::SingleVal::new(super::super::ID::CMDHistory(super::ID::CMDAmend(ID::ValEventId)));
        const VAL_EVENT_ID: Arg<GlobalID> = Arg {
            id: ID_VAL_EVENT_ID.into(),
            max_values: 1,
            possible_values: &[],
        };
        pub const ID_VAL_ARGS: id::MultiVal<GlobalID> = id::MultiVal::new(super::super::ID::CMDHistory(super::ID::CMDAmend(ID::ValArgs)));
        const VAL_ARGS: Arg<GlobalID> = Arg {
            id: ID_VAL_ARGS.into(),
            max_values: 18446744073709551615,
            possible_values: &[],
        };
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum ID {
            ValEventId,
            ValArgs,
            ValEnv,
        }
        pub(super) const CMD: Command<GlobalID> = Command {
            name: "amend",
            description: "",
            all_flags: &[VAL_ENV, VAL_NO_ENV, super::super::VAL_NO_TRACE, super::super::VAL_HUMBLE, super::super::VAL_ARCHAEOLOGY, super::super::VAL_SELECT, super::super::VAL_ALL, super::super::VAL_RECENT, super::super::VAL_TIMELESS],
            args: &[VAL_EVENT_ID, VAL_ARGS],
            commands: &[],
        };
    }
    pub mod tidy {
        use super::super::GlobalID as GlobalID;
        use supplement::core::*;

        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum ID {
        }
        pub(super) const CMD: Command<GlobalID> = Command {
            name: "tidy",
            description: "",
            all_flags: &[super::super::VAL_NO_TRACE, super::super::VAL_HUMBLE, super::super::VAL_ARCHAEOLOGY, super::super::VAL_SELECT, super::super::VAL_ALL, super::super::VAL_RECENT, super::super::VAL_TIMELESS],
            args: &[],
            commands: &[],
        };
    }
}
pub mod top {
    use super::GlobalID as GlobalID;
    use supplement::core::*;

    pub const ID_VAL_WAIT: id::NoVal = id::NoVal::new_certain(line!());
    const VAL_WAIT: Flag<GlobalID> = Flag {
        short: &['w'],
        long: &["wait"],
        description: "Wait for all involved processes to halt",
        once: true,
        ty: flag_type::Type::new_bool(ID_VAL_WAIT),
    };
    pub const ID_VAL_ID: id::MultiVal<GlobalID> = id::MultiVal::new(super::ID::CMDTop(ID::ValId));
    const VAL_ID: Flag<GlobalID> = Flag {
        short: &[],
        long: &["id"],
        description: "Run event ID",
        once: false,
        ty: flag_type::Type::new_valued(ID_VAL_ID.into(), CompleteWithEqual::NoNeed, &[]),
    };
    pub const ID_VAL_QUERIES: id::MultiVal<GlobalID> = id::MultiVal::new(super::ID::CMDTop(ID::ValQueries));
    const VAL_QUERIES: Arg<GlobalID> = Arg {
        id: ID_VAL_QUERIES.into(),
        max_values: 18446744073709551615,
        possible_values: &[],
    };
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum ID {
        ValQueries,
        ValId,
    }
    pub(super) const CMD: Command<GlobalID> = Command {
        name: "top",
        description: "Monitor hs process",
        all_flags: &[VAL_WAIT, VAL_ID, super::VAL_NO_TRACE, super::VAL_HUMBLE, super::VAL_ARCHAEOLOGY, super::VAL_SELECT, super::VAL_ALL, super::VAL_RECENT, super::VAL_TIMELESS],
        args: &[VAL_QUERIES],
        commands: &[],
    };
}
