function __hs_list_named_filters
    string split ' ' (hs --no-alias tags ls --named)
end

function __hs_list_tags
    # TODO: different home?
    if [ "$argv" = "append" ]
        set append 1
    end
    if set -q append
        echo "+all"
    else
        echo "all"
    end
    for tag in (string split ' ' (hs --no-alias tags ls --known))
        if set -q append
            echo +$tag
        else
            echo $tag
        end
    end
end

function __hs_list_scripts
    set orig_cmd (commandline -j)
    set cmd_arr (string split ' ' $orig_cmd)
    if echo $cmd_arr[-1] | string match -q -r ".*!\$"
        set bang 1
        set cmd "hs -f all --timeless"
    else
        set cmd (eval "command hs completion alias $orig_cmd" 2>/dev/null)
        if [ $status -ne 0 ]
            return
        end
    end
    
    set list (eval "command hs completion ls $cmd" 2>/dev/null)
    if [ $status -ne 0 ]
        return
    end
    for script in (string split ' ' $list)
        if set -q bang
            echo $script!
        else
            echo $script
        end
    end
end

complete -c hs -a "(__hs_list_scripts)"

complete -c hs -n "__fish_use_subcommand" -s H -l hs-home -d 'Path to hyper script home'
complete -c hs -n "__fish_use_subcommand" -s f -l filter -d 'Filter by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags)"
complete -c hs -n "__fish_use_subcommand" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_use_subcommand" -l prompt-level -d 'Prompt level of fuzzy finder.' -r -f -a "never always smart on-multi-fuzz"
complete -c hs -n "__fish_use_subcommand" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_use_subcommand" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_use_subcommand" -s a -l all -d 'Shorthand for `-f=all,^removed --timeless`'
complete -c hs -n "__fish_use_subcommand" -l timeless -d 'Show scripts of all time.'
complete -c hs -n "__fish_use_subcommand" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_use_subcommand" -s V -l version -d 'Prints version information'
complete -c hs -n "__fish_use_subcommand" -f -a "help" -d 'Prints this message, the help of the given subcommand(s), or a script\'s help message.'
complete -c hs -n "__fish_use_subcommand" -f -a "edit" -d 'Edit hyper script'
complete -c hs -n "__fish_use_subcommand" -f -a "alias" -d 'Manage alias'
complete -c hs -n "__fish_use_subcommand" -f -a "run" -d 'Run the script'
complete -c hs -n "__fish_use_subcommand" -f -a "which" -d 'Execute the script query and get the exact file'
complete -c hs -n "__fish_use_subcommand" -f -a "cat" -d 'Print the script to standard output'
complete -c hs -n "__fish_use_subcommand" -f -a "rm" -d 'Remove the script'
complete -c hs -n "__fish_use_subcommand" -f -a "ls" -d 'List hyper scripts'
complete -c hs -n "__fish_use_subcommand" -f -a "cp" -d 'Copy the script to another one'
complete -c hs -n "__fish_use_subcommand" -f -a "mv" -d 'Move the script to another one'
complete -c hs -n "__fish_use_subcommand" -f -a "tags" -d 'Manage script tags. If a tag filter is given, store it to config, otherwise show tag information.'
complete -c hs -n "__fish_use_subcommand" -f -a "history" -d 'Manage script history'

complete -c hs -n "__fish_seen_subcommand_from help" -s f -l filter -d 'Filter by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags)"
complete -c hs -n "__fish_seen_subcommand_from help" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from help" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from help" -s V -l version -d 'Prints version information'
complete -c hs -n "__fish_seen_subcommand_from help" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from help" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from help" -s a -l all -d 'Shorthand for `-f=all,^removed --timeless`'
complete -c hs -n "__fish_seen_subcommand_from help" -l timeless -d 'Show scripts of all time.'

complete -c hs -n "__fish_seen_subcommand_from edit" -s T -l ty -d 'Type of the script, e.g. `sh`'
complete -c hs -n "__fish_seen_subcommand_from edit" -s t -l tags
complete -c hs -n "__fish_seen_subcommand_from edit" -s f -l filter -d 'Filter by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags)"
complete -c hs -n "__fish_seen_subcommand_from edit" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from edit" -s n -l no-template
complete -c hs -n "__fish_seen_subcommand_from edit" -l fast -d 'Create script without invoking the editor'
complete -c hs -n "__fish_seen_subcommand_from edit" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from edit" -s V -l version -d 'Prints version information'
complete -c hs -n "__fish_seen_subcommand_from edit" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from edit" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from edit" -s a -l all -d 'Shorthand for `-f=all,^removed --timeless`'
complete -c hs -n "__fish_seen_subcommand_from edit" -l timeless -d 'Show scripts of all time.'

complete -c hs -n "__fish_seen_subcommand_from alias" -s f -l filter -d 'Filter by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags)"
complete -c hs -n "__fish_seen_subcommand_from alias" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from alias" -s u -l unset -d 'Unset an alias.'
complete -c hs -n "__fish_seen_subcommand_from alias" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from alias" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from alias" -s a -l all -d 'Shorthand for `-f=all,^removed --timeless`'
complete -c hs -n "__fish_seen_subcommand_from alias" -l timeless -d 'Show scripts of all time.'
complete -c hs -n "__fish_seen_subcommand_from run" -s r -l repeat
complete -c hs -n "__fish_seen_subcommand_from run" -s f -l filter -d 'Filter by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags)"
complete -c hs -n "__fish_seen_subcommand_from run" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from run" -l dummy -d 'Add a dummy run history instead of actually running it'
complete -c hs -n "__fish_seen_subcommand_from run" -s p -l previous-args
complete -c hs -n "__fish_seen_subcommand_from run" -s d -l dir
complete -c hs -n "__fish_seen_subcommand_from run" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from run" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from run" -s a -l all -d 'Shorthand for `-f=all,^removed --timeless`'
complete -c hs -n "__fish_seen_subcommand_from run" -l timeless -d 'Show scripts of all time.'
complete -c hs -n "__fish_seen_subcommand_from which" -s f -l filter -d 'Filter by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags)"
complete -c hs -n "__fish_seen_subcommand_from which" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from which" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from which" -s V -l version -d 'Prints version information'
complete -c hs -n "__fish_seen_subcommand_from which" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from which" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from which" -s a -l all -d 'Shorthand for `-f=all,^removed --timeless`'
complete -c hs -n "__fish_seen_subcommand_from which" -l timeless -d 'Show scripts of all time.'
complete -c hs -n "__fish_seen_subcommand_from cat" -s f -l filter -d 'Filter by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags)"
complete -c hs -n "__fish_seen_subcommand_from cat" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from cat" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from cat" -s V -l version -d 'Prints version information'
complete -c hs -n "__fish_seen_subcommand_from cat" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from cat" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from cat" -s a -l all -d 'Shorthand for `-f=all,^removed --timeless`'
complete -c hs -n "__fish_seen_subcommand_from cat" -l timeless -d 'Show scripts of all time.'

complete -c hs -n "__fish_seen_subcommand_from rm" -s f -l filter -d 'Filter by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags)"
complete -c hs -n "__fish_seen_subcommand_from rm" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from rm" -l purge -d 'Actually remove scripts, rather than hiding them with tag.'
complete -c hs -n "__fish_seen_subcommand_from rm" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from rm" -s V -l version -d 'Prints version information'
complete -c hs -n "__fish_seen_subcommand_from rm" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from rm" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from rm" -s a -l all -d 'Shorthand for `-f=all,^removed --timeless`'
complete -c hs -n "__fish_seen_subcommand_from rm" -l timeless -d 'Show scripts of all time.'

complete -c hs -n "__fish_seen_subcommand_from ls" -l grouping -d 'Grouping style.' -r -f -a "tag tree none"
complete -c hs -n "__fish_seen_subcommand_from ls" -s f -l filter -d 'Filter by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags)"
complete -c hs -n "__fish_seen_subcommand_from ls" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from ls" -s l -l long -d 'Show verbose information.'
complete -c hs -n "__fish_seen_subcommand_from ls" -l plain -d 'No color and other decoration.'
complete -c hs -n "__fish_seen_subcommand_from ls" -l file -d 'Show file path to the script.'
complete -c hs -n "__fish_seen_subcommand_from ls" -l name -d 'Show name of the script.'
complete -c hs -n "__fish_seen_subcommand_from ls" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from ls" -s V -l version -d 'Prints version information'
complete -c hs -n "__fish_seen_subcommand_from ls" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from ls" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from ls" -s a -l all -d 'Shorthand for `-f=all,^removed --timeless`'
complete -c hs -n "__fish_seen_subcommand_from ls" -l timeless -d 'Show scripts of all time.'
complete -c hs -n "__fish_seen_subcommand_from cp" -s t -l tags
complete -c hs -n "__fish_seen_subcommand_from cp" -s f -l filter -d 'Filter by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags)"
complete -c hs -n "__fish_seen_subcommand_from cp" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from cp" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from cp" -s V -l version -d 'Prints version information'
complete -c hs -n "__fish_seen_subcommand_from cp" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from cp" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from cp" -s a -l all -d 'Shorthand for `-f=all,^removed --timeless`'
complete -c hs -n "__fish_seen_subcommand_from cp" -l timeless -d 'Show scripts of all time.'
complete -c hs -n "__fish_seen_subcommand_from mv" -s T -l ty -d 'Type of the script, e.g. `sh`'
complete -c hs -n "__fish_seen_subcommand_from mv" -s t -l tags
complete -c hs -n "__fish_seen_subcommand_from mv" -s f -l filter -d 'Filter by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags)"
complete -c hs -n "__fish_seen_subcommand_from mv" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from mv" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from mv" -s V -l version -d 'Prints version information'
complete -c hs -n "__fish_seen_subcommand_from mv" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from mv" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from mv" -s a -l all -d 'Shorthand for `-f=all,^removed --timeless`'
complete -c hs -n "__fish_seen_subcommand_from mv" -l timeless -d 'Show scripts of all time.'

complete -c hs -n "__fish_seen_subcommand_from tags" -s f -l filter -d 'Filter by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags)"
complete -c hs -n "__fish_seen_subcommand_from tags" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from tags" -s V -l version -d 'Prints version information'
complete -c hs -n "__fish_seen_subcommand_from tags" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from tags" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from tags" -s a -l all -d 'Shorthand for `-f=all,^removed --timeless`'
complete -c hs -n "__fish_seen_subcommand_from tags" -l timeless -d 'Show scripts of all time.'
complete -c hs -n "__fish_prev_arg_in tags" -f -a "unset"
complete -c hs -n "__fish_prev_arg_in tags" -f -a "set"
complete -c hs -n "__fish_prev_arg_in tags" -f -a "ls"
complete -c hs -n "__fish_prev_arg_in tags" -f -a "toggle"
complete -c hs -n "__fish_seen_subcommand_from tags" -s n -l name
complete -c hs -n "__fish_seen_subcommand_from set" -s n -l name
complete -c hs -n "__fish_seen_subcommand_from ls" -s k -l known # FIXME: 這會補到另一個 ls 上 =_=
complete -c hs -n "__fish_prev_arg_in tags" -f -a "(__hs_list_tags append)"
complete -c hs -n "__fish_seen_subcommand_from set" -f -a "(__hs_list_tags append)"
complete -c hs -n "__fish_seen_subcommand_from set" -f -a "(__hs_list_tags append)"
complete -c hs -n "__fish_seen_subcommand_from unset" -f -a "(__hs_list_named_filters)"
complete -c hs -n "__fish_seen_subcommand_from toggle" -f -a "(__hs_list_named_filters)"

complete -c hs -n "__fish_seen_subcommand_from history" -s f -l filter -d 'Filter by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags)"
complete -c hs -n "__fish_seen_subcommand_from history" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from history" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from history" -s V -l version -d 'Prints version information'
complete -c hs -n "__fish_seen_subcommand_from history" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from history" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from history" -s a -l all -d 'Shorthand for `-f=all,^removed --timeless`'
complete -c hs -n "__fish_prev_arg_in history" -l timeless -d 'Show scripts of all time.'
complete -c hs -n "__fish_prev_arg_in history" -f -a "rm"
complete -c hs -n "__fish_prev_arg_in history" -f -a "rm-id" -d 'Remove history by the event\'s id
Useful if you want to keep those illegal arguments from polluting the history.'
complete -c hs -n "__fish_prev_arg_in history" -f -a "show"
complete -c hs -n "__fish_prev_arg_in history" -f -a "neglect"
complete -c hs -n "__fish_prev_arg_in history" -f -a "amend"
complete -c hs -n "__fish_prev_arg_in history" -f -a "tidy"
complete -c hs -n "__fish_seen_subcommand_from rm" -s f -l filter -d 'Filter by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags)"
complete -c hs -n "__fish_seen_subcommand_from rm" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from rm" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from rm" -s V -l version -d 'Prints version information'
complete -c hs -n "__fish_seen_subcommand_from rm" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from rm" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from rm" -s a -l all -d 'Shorthand for `-f=all,^removed --timeless`'
complete -c hs -n "__fish_seen_subcommand_from rm" -l timeless -d 'Show scripts of all time.'
complete -c hs -n "__fish_seen_subcommand_from rm-id" -s f -l filter -d 'Filter by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags)"
complete -c hs -n "__fish_seen_subcommand_from rm-id" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from rm-id" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from rm-id" -s V -l version -d 'Prints version information'
complete -c hs -n "__fish_seen_subcommand_from rm-id" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from rm-id" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from rm-id" -s a -l all -d 'Shorthand for `-f=all,^removed --timeless`'
complete -c hs -n "__fish_seen_subcommand_from rm-id" -l timeless -d 'Show scripts of all time.'
complete -c hs -n "__fish_seen_subcommand_from show" -s l -l limit
complete -c hs -n "__fish_seen_subcommand_from show" -s o -l offset
complete -c hs -n "__fish_seen_subcommand_from show" -s d -l dir
complete -c hs -n "__fish_seen_subcommand_from show" -s f -l filter -d 'Filter by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags)"
complete -c hs -n "__fish_seen_subcommand_from show" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from show" -l with-name
complete -c hs -n "__fish_seen_subcommand_from show" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from show" -s V -l version -d 'Prints version information'
complete -c hs -n "__fish_seen_subcommand_from show" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from show" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from show" -s a -l all -d 'Shorthand for `-f=all,^removed --timeless`'
complete -c hs -n "__fish_seen_subcommand_from show" -l timeless -d 'Show scripts of all time.'
complete -c hs -n "__fish_seen_subcommand_from neglect" -s f -l filter -d 'Filter by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags)"
complete -c hs -n "__fish_seen_subcommand_from neglect" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from neglect" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from neglect" -s V -l version -d 'Prints version information'
complete -c hs -n "__fish_seen_subcommand_from neglect" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from neglect" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from neglect" -s a -l all -d 'Shorthand for `-f=all,^removed --timeless`'
complete -c hs -n "__fish_seen_subcommand_from neglect" -l timeless -d 'Show scripts of all time.'
complete -c hs -n "__fish_seen_subcommand_from amend" -s f -l filter -d 'Filter by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags)"
complete -c hs -n "__fish_seen_subcommand_from amend" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from amend" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from amend" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from amend" -s a -l all -d 'Shorthand for `-f=all,^removed --timeless`'
complete -c hs -n "__fish_seen_subcommand_from amend" -l timeless -d 'Show scripts of all time.'
complete -c hs -n "__fish_seen_subcommand_from tidy" -s f -l filter -d 'Filter by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags)"
complete -c hs -n "__fish_seen_subcommand_from tidy" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from tidy" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from tidy" -s V -l version -d 'Prints version information'
complete -c hs -n "__fish_seen_subcommand_from tidy" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from tidy" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from tidy" -s a -l all -d 'Shorthand for `-f=all,^removed --timeless`'
complete -c hs -n "__fish_seen_subcommand_from tidy" -l timeless -d 'Show scripts of all time.'
