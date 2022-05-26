#compdef hs

autoload -U is-at-least

_hs() {
    typeset -A opt_args
    typeset -a _arguments_options
    local ret=1

    if is-at-least 5.2; then
        _arguments_options=(-s -S -C)
    else
        _arguments_options=(-s -C)
    fi

    local context curcontext="$curcontext" state line
    _arguments "${_arguments_options[@]}" \
'-H+[Path to hyper script home]:HS_HOME: ' \
'--hs-home=[Path to hyper script home]:HS_HOME: ' \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--toggle=[Toggle named selector temporarily]:TOGGLE: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'--prompt-level=[Prompt level of fuzzy finder.]:PROMPT_LEVEL:(never always smart on-multi-fuzz)' \
'-h[Print help information]' \
'--help[Print help information]' \
'-V[Print version information]' \
'--version[Print version information]' \
'--dump-args[]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'--no-alias[]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
":: :_hs_commands" \
"*::: :->hyper-scripter" \
&& ret=0
    case $state in
    (hyper-scripter)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:hs-command-$line[1]:"
        case $line[1] in
            (help)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
'*::args:' \
&& ret=0
;;
(env-help)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
'::script-query -- Target script.
`-` or `^{N}` for previous script, and `={NAME}` for exact name matching.
Otherwise, do fuzzy search.:' \
&& ret=0
;;
(load-utils)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
&& ret=0
;;
(migrate)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
&& ret=0
;;
(edit)
_arguments "${_arguments_options[@]}" \
'-T+[Type of the script, e.g. `sh`]:TY: ' \
'--ty=[Type of the script, e.g. `sh`]:TY: ' \
'-t+[Tags of the script]:TAGS: ' \
'--tags=[Tags of the script]:TAGS: ' \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'-n[]' \
'--no-template[]' \
'--fast[Create script without invoking the editor]' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
'::edit-query -- Target script.
`?` for new anonymous, `-` or `^{N}` for previous script, and `={NAME}` for exact name matching.
Otherwise, do fuzzy search.:' \
'*::content -- Because the field `content` is rarely used, don'\''t make it allow hyphen value Otherwise, options like `-T e` will be absorbed if placed after script query:' \
&& ret=0
;;
(alias)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'()--short[]' \
'()-u[Unset an alias.]' \
'()--unset[Unset an alias.]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
'::before:' \
'*::after:' \
&& ret=0
;;
(run)
_arguments "${_arguments_options[@]}" \
'-r+[]:REPEAT: ' \
'--repeat=[]:REPEAT: ' \
'-d+[]:DIR: ' \
'--dir=[]:DIR: ' \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'--dummy[Add a dummy run history instead of actually running it]' \
'-p[Use arguments from last run]' \
'--previous-args[Use arguments from last run]' \
'-E[Raise an error if --previous-args is given but there is no previous argument]' \
'--error-no-previous[Raise an error if --previous-args is given but there is no previous argument]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
'::script-query -- Target script.
`-` or `^{N}` for previous script, and `={NAME}` for exact name matching.
Otherwise, do fuzzy search.:' \
'*::args -- Command line args to pass to the script:' \
&& ret=0
;;
(which)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
'::script-query -- Target script.
`-` or `^{N}` for previous script, and `={NAME}` for exact name matching.
Otherwise, do fuzzy search.:' \
&& ret=0
;;
(cat)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
'::script-query -- Target script.
`-` or `^{N}` for previous script, and `={NAME}` for exact name matching.
Otherwise, do fuzzy search.:' \
&& ret=0
;;
(rm)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'--purge[Actually remove scripts, rather than hiding them with tag.]' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
'*::queries -- Target script.
`-` or `^{N}` for previous script, and `={NAME}` for exact name matching.
Otherwise, do fuzzy search.
Wildcard such as name/* is also allowed.:' \
&& ret=0
;;
(ls)
_arguments "${_arguments_options[@]}" \
'--grouping=[Grouping style.]:GROUPING:(tag tree none)' \
'--limit=[Limit the amount of scripts found.]:LIMIT: ' \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'-l[Show verbose information.]' \
'--long[Show verbose information.]' \
'--plain[No color and other decoration.]' \
'(-l --long)--file[Show file path to the script.]' \
'(-l --long)--name[Show name of the script.]' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
'*::queries -- Target script.
`-` or `^{N}` for previous script, and `={NAME}` for exact name matching.
Otherwise, do fuzzy search.
Wildcard such as name/* is also allowed.:' \
&& ret=0
;;
(types)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
":: :_hs__types_commands" \
"*::: :->types" \
&& ret=0

    case $state in
    (types)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:hs-types-command-$line[1]:"
        case $line[1] in
            (ls)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'--version[Print version information]' \
'--no-sub[]' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
&& ret=0
;;
(template)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'--version[Print version information]' \
'-e[]' \
'--edit[]' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
':ty -- Type of the script, e.g. `sh`:' \
&& ret=0
;;
        esac
    ;;
esac
;;
(cp)
_arguments "${_arguments_options[@]}" \
'-t+[Tags of the script]:TAGS: ' \
'--tags=[Tags of the script]:TAGS: ' \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
':origin -- Target script.
`-` or `^{N}` for previous script, and `={NAME}` for exact name matching.
Otherwise, do fuzzy search.:' \
':new -- New script. `?` for new anonymous.:' \
&& ret=0
;;
(mv)
_arguments "${_arguments_options[@]}" \
'-T+[Type of the script, e.g. `sh`]:TY: ' \
'--ty=[Type of the script, e.g. `sh`]:TY: ' \
'-t+[Tags of the script]:TAGS: ' \
'--tags=[Tags of the script]:TAGS: ' \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
':origin -- Target script.
`-` or `^{N}` for previous script, and `={NAME}` for exact name matching.
Otherwise, do fuzzy search.
Wildcard such as name/* is also allowed.:' \
'::new -- New script. `?` for new anonymous.:' \
&& ret=0
;;
(tags)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
":: :_hs__tags_commands" \
"*::: :->tags" \
&& ret=0

    case $state in
    (tags)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:hs-tags-command-$line[1]:"
        case $line[1] in
            (unset)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'--version[Print version information]' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
':name:' \
&& ret=0
;;
(set)
_arguments "${_arguments_options[@]}" \
'-n+[]:NAME: ' \
'--name=[]:NAME: ' \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'--version[Print version information]' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
':content:' \
&& ret=0
;;
(ls)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'--version[Print version information]' \
'-k[]' \
'--known[]' \
'(-k --known)-n[]' \
'(-k --known)--named[]' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
&& ret=0
;;
(toggle)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'--version[Print version information]' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
':name:' \
&& ret=0
;;
        esac
    ;;
esac
;;
(history)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
":: :_hs__history_commands" \
"*::: :->history" \
&& ret=0

    case $state in
    (history)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:hs-history-command-$line[1]:"
        case $line[1] in
            (rm)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'--version[Print version information]' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
'*::queries -- Target script.
`-` or `^{N}` for previous script, and `={NAME}` for exact name matching.
Otherwise, do fuzzy search.
Wildcard such as name/* is also allowed.:' \
':range:' \
&& ret=0
;;
(rm-id)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'--version[Print version information]' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
':event-id:' \
&& ret=0
;;
(humble)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'--version[Print version information]' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
':event-id:' \
&& ret=0
;;
(show)
_arguments "${_arguments_options[@]}" \
'-l+[]:LIMIT: ' \
'--limit=[]:LIMIT: ' \
'-o+[]:OFFSET: ' \
'--offset=[]:OFFSET: ' \
'-d+[]:DIR: ' \
'--dir=[]:DIR: ' \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'--version[Print version information]' \
'--with-name[]' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
'*::queries -- Target script.
`-` or `^{N}` for previous script, and `={NAME}` for exact name matching.
Otherwise, do fuzzy search.
Wildcard such as name/* is also allowed.:' \
&& ret=0
;;
(neglect)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'--version[Print version information]' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
'*::queries -- Target script.
`-` or `^{N}` for previous script, and `={NAME}` for exact name matching.
Otherwise, do fuzzy search.
Wildcard such as name/* is also allowed.:' \
&& ret=0
;;
(amend)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'--version[Print version information]' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
':event-id:' \
'*::args -- Command line args to pass to the script:' \
&& ret=0
;;
(tidy)
_arguments "${_arguments_options[@]}" \
'(-a --all)*-s+[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'(-a --all)*--select=[Select by tags, e.g. `all,^remove`]:SELECT: ' \
'--recent=[Show scripts within recent days.]:RECENT: ' \
'--version[Print version information]' \
'-h[Print help information]' \
'--help[Print help information]' \
'--no-trace[Don'\''t record history]' \
'(--no-trace)--humble[Don'\''t affect script time order (but still record history and affect time filter)]' \
'(-a --all --timeless)-A[Show scripts NOT within recent days]' \
'(-a --all --timeless)--archaeology[Show scripts NOT within recent days]' \
'(--recent)-a[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--all[Shorthand for `-s=all,^remove --timeless`]' \
'(--recent)--timeless[Show scripts of all time.]' \
'*::queries -- Target script.
`-` or `^{N}` for previous script, and `={NAME}` for exact name matching.
Otherwise, do fuzzy search.
Wildcard such as name/* is also allowed.:' \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
}

(( $+functions[_hs_commands] )) ||
_hs_commands() {
    local commands; commands=(
'help:Prints this message, the help of the given subcommand(s), or a script'\''s help message.' \
'env-help:Print the help message of env variables' \
'load-utils:' \
'migrate:Migrate the database' \
'edit:Edit hyper script' \
'alias:Manage alias' \
'run:Run the script' \
'which:Execute the script query and get the exact file' \
'cat:Print the script to standard output' \
'rm:Remove the script' \
'ls:List hyper scripts' \
'types:Manage script types' \
'cp:Copy the script to another one' \
'mv:Move the script to another one' \
'tags:Manage script tags' \
'history:Manage script history' \
    )
    _describe -t commands 'hs commands' commands "$@"
}
(( $+functions[_hs__alias_commands] )) ||
_hs__alias_commands() {
    local commands; commands=()
    _describe -t commands 'hs alias commands' commands "$@"
}
(( $+functions[_hs__history__amend_commands] )) ||
_hs__history__amend_commands() {
    local commands; commands=()
    _describe -t commands 'hs history amend commands' commands "$@"
}
(( $+functions[_hs__cat_commands] )) ||
_hs__cat_commands() {
    local commands; commands=()
    _describe -t commands 'hs cat commands' commands "$@"
}
(( $+functions[_hs__cp_commands] )) ||
_hs__cp_commands() {
    local commands; commands=()
    _describe -t commands 'hs cp commands' commands "$@"
}
(( $+functions[_hs__edit_commands] )) ||
_hs__edit_commands() {
    local commands; commands=()
    _describe -t commands 'hs edit commands' commands "$@"
}
(( $+functions[_hs__env-help_commands] )) ||
_hs__env-help_commands() {
    local commands; commands=()
    _describe -t commands 'hs env-help commands' commands "$@"
}
(( $+functions[_hs__help_commands] )) ||
_hs__help_commands() {
    local commands; commands=()
    _describe -t commands 'hs help commands' commands "$@"
}
(( $+functions[_hs__history_commands] )) ||
_hs__history_commands() {
    local commands; commands=(
'rm:' \
'rm-id:Remove an event by it'\''s id.
Useful if you want to keep those illegal arguments from polluting the history.' \
'humble:Humble an event by it'\''s id' \
'show:' \
'neglect:' \
'amend:' \
'tidy:' \
    )
    _describe -t commands 'hs history commands' commands "$@"
}
(( $+functions[_hs__history__humble_commands] )) ||
_hs__history__humble_commands() {
    local commands; commands=()
    _describe -t commands 'hs history humble commands' commands "$@"
}
(( $+functions[_hs__load-utils_commands] )) ||
_hs__load-utils_commands() {
    local commands; commands=()
    _describe -t commands 'hs load-utils commands' commands "$@"
}
(( $+functions[_hs__ls_commands] )) ||
_hs__ls_commands() {
    local commands; commands=()
    _describe -t commands 'hs ls commands' commands "$@"
}
(( $+functions[_hs__tags__ls_commands] )) ||
_hs__tags__ls_commands() {
    local commands; commands=()
    _describe -t commands 'hs tags ls commands' commands "$@"
}
(( $+functions[_hs__types__ls_commands] )) ||
_hs__types__ls_commands() {
    local commands; commands=()
    _describe -t commands 'hs types ls commands' commands "$@"
}
(( $+functions[_hs__migrate_commands] )) ||
_hs__migrate_commands() {
    local commands; commands=()
    _describe -t commands 'hs migrate commands' commands "$@"
}
(( $+functions[_hs__mv_commands] )) ||
_hs__mv_commands() {
    local commands; commands=()
    _describe -t commands 'hs mv commands' commands "$@"
}
(( $+functions[_hs__history__neglect_commands] )) ||
_hs__history__neglect_commands() {
    local commands; commands=()
    _describe -t commands 'hs history neglect commands' commands "$@"
}
(( $+functions[_hs__history__rm_commands] )) ||
_hs__history__rm_commands() {
    local commands; commands=()
    _describe -t commands 'hs history rm commands' commands "$@"
}
(( $+functions[_hs__rm_commands] )) ||
_hs__rm_commands() {
    local commands; commands=()
    _describe -t commands 'hs rm commands' commands "$@"
}
(( $+functions[_hs__history__rm-id_commands] )) ||
_hs__history__rm-id_commands() {
    local commands; commands=()
    _describe -t commands 'hs history rm-id commands' commands "$@"
}
(( $+functions[_hs__run_commands] )) ||
_hs__run_commands() {
    local commands; commands=()
    _describe -t commands 'hs run commands' commands "$@"
}
(( $+functions[_hs__tags__set_commands] )) ||
_hs__tags__set_commands() {
    local commands; commands=()
    _describe -t commands 'hs tags set commands' commands "$@"
}
(( $+functions[_hs__history__show_commands] )) ||
_hs__history__show_commands() {
    local commands; commands=()
    _describe -t commands 'hs history show commands' commands "$@"
}
(( $+functions[_hs__tags_commands] )) ||
_hs__tags_commands() {
    local commands; commands=(
'unset:' \
'set:' \
'ls:' \
'toggle:' \
    )
    _describe -t commands 'hs tags commands' commands "$@"
}
(( $+functions[_hs__types__template_commands] )) ||
_hs__types__template_commands() {
    local commands; commands=()
    _describe -t commands 'hs types template commands' commands "$@"
}
(( $+functions[_hs__history__tidy_commands] )) ||
_hs__history__tidy_commands() {
    local commands; commands=()
    _describe -t commands 'hs history tidy commands' commands "$@"
}
(( $+functions[_hs__tags__toggle_commands] )) ||
_hs__tags__toggle_commands() {
    local commands; commands=()
    _describe -t commands 'hs tags toggle commands' commands "$@"
}
(( $+functions[_hs__types_commands] )) ||
_hs__types_commands() {
    local commands; commands=(
'ls:' \
'template:' \
    )
    _describe -t commands 'hs types commands' commands "$@"
}
(( $+functions[_hs__tags__unset_commands] )) ||
_hs__tags__unset_commands() {
    local commands; commands=()
    _describe -t commands 'hs tags unset commands' commands "$@"
}
(( $+functions[_hs__which_commands] )) ||
_hs__which_commands() {
    local commands; commands=()
    _describe -t commands 'hs which commands' commands "$@"
}

_hs "$@"
