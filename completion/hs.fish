function __hs_extract_home_and_run
    set cmd (commandline -j)
    set hs_home (eval "command hs completion home -- $cmd" 2>/dev/null)
    if [ $status -eq 0 ]
        if [ ! -d $hs_home ]
            # Stop completion cause we don't want to create directory
            return 1
        end
        set home_args "-H $hs_home"
    end
    eval "command hs --no-alias $home_args $argv" 2>/dev/null
end

function __hs_list_types
    string split ' ' (__hs_extract_home_and_run types ls $argv)
end

function __hs_expand_alias
    set cmd (eval "command hs completion alias -- $argv" 2>/dev/null)
    if [ $status -eq 0 ]
        echo $cmd
    else
        echo $argv
    end
end

function __hs_list_named_selectors
    string split ' ' (__hs_extract_home_and_run tags ls --named)
end

function __hs_list_tags
    set bang ""
    if [ "$argv" = "append" ]
        set append 1
    else if [ "$argv" = "both" ]
        set append 1
        set no_append 1
        set bang !
    else
        set no_append 1
    end

    for tag in (string split ' ' (__hs_extract_home_and_run tags ls --known) remove all)
        if set -q append
            echo +$tag$bang\t+$tag$bang
        end
        if set -q no_append
            echo $tag$bang\t$tag$bang
        end
    end
end

function __hs_list_tags_and_types
    __hs_list_tags $argv

    if [ "$argv" = "append" ]
        set append 1
    else if [ "$argv" = "both" ]
        set append 1
        set no_append 1
        set bang !
    else
        set no_append 1
    end

    for ty in (string split ' ' (__hs_extract_home_and_run types ls --no-sub))
        if set -q append
            echo +@$ty$bang\t+$ty$bang
        end
        if set -q no_append
            echo @$ty$bang\t$ty$bang
        end
    end
end

function __hs_list_scripts
    set orig_cmd (commandline -j)
    set cmd_arr (string split ' ' $orig_cmd)

    set name $cmd_arr[-1]

    if echo $name | string match -q -r ".*!\$"
        set bang "!"
        set bang_arg "--bang"
    end

    if echo $name | string match -q -r "^\^.*" || [ $name = "!" ] || [ ! -n $name ]
        # NOTE: Special case. Latest script completion.
        # TODO: Make it support anonymous script
        set cmd "$cmd_arr[1..-1] trailing"
        set list (eval "command hs completion ls $bang_arg --limit 10 -- $cmd" 2>/dev/null)
        if [ $status -ne 0 ]
            return
        end
        set num 0
        for script in (string split ' ' $list)
            set num (math $num + 1)
            set bang "$bang"
            echo $script$bang\t^$num$bang
        end
        return
    end

    if set -q bang
        set name (string replace ! '' $name)
    end
    if echo $name | string match -q -r "=.*\$"
        set exact "="
        set name (string replace = '' $name)
    end
    if [ -n $name ]
        set name_arg "--name $name"
    end
    set cmd "$cmd_arr[1..-2] trailing"

    set list (eval "command hs completion ls $bang_arg $name_arg -- $cmd" 2>/dev/null)
    if [ $status -ne 0 ]
        return
    end

    for script in (string split ' ' $list)
        set res "$exact$script$bang"
        # NOTE: duplicate the script name to mimic the "reorder fuzzy search"
        echo $res\t$res
    end
end

function __hs_not_run_arg_or_alias
    __hs_is_alias
    if [ $status -eq 0 ]
        return 1
    end

    set orig_cmd (commandline -j)
    set cmd_arr (string split ' ' $orig_cmd)

    if [ -z "$cmd_arr[-1]" ]
        # pad one word to the end of command. 
        set trailing "trailing"
    end

    set run_args (eval "command hs completion parse-run -- $orig_cmd $trailing" 2>/dev/null)
    if [ $status -ne 0 ]
        return 0
    end

    set run_args_arr (string split ' ' $run_args)
    if [ -z "$run_args_arr[2]" ]
        return 0
    else
        return 1
    end
end

function __hs_list_alias
    set cmd_arr (string split ' ' (commandline -j))
    if [ $cmd_arr[2] = '--no-alias' ]
        return
    end

    set list (__hs_extract_home_and_run alias)
    for line in $list
        set arr (string split \t $line)
        echo $arr[1]
    end
end

function __hs_is_alias
    set cmd (commandline -j)
    set cmd_arr (string split ' ' $cmd)
    if [ -n "$cmd_arr[-1]" ]
        # remove the last argument
        set cmd "$cmd_arr[1..-2]"
    end
    eval "command hs completion alias -- $cmd" 2>/dev/null
end

function __hs_alias_completion
    set orig_cmd (commandline -j)
    set cmd (__hs_expand_alias $orig_cmd)

    set orig_cmd_arr (string split ' ' $orig_cmd)
    if [ -z "$orig_cmd_arr[-1]" ]
        # preserve the last white space
        set space ' '
    end
    set cmd_arr (string split ' ' $cmd)

    complete -C "hs --no-alias $cmd_arr[2..]$space"
end

complete -k -c hs -n "__hs_is_alias" -x -a "(__hs_alias_completion)"

complete -k -c hs -n "__hs_not_run_arg_or_alias" -x -a "(__hs_list_scripts)"

function __hs_use_subcommand
    set cmd (commandline -j)
    set cmd_arr (string split ' ' $cmd)
    if [ -n "$cmd_arr[-1]" ]
        # remove the last argument
        set cmd "$cmd_arr[1..-2]"
    end
    eval "command hs completion no-subcommand -- $cmd" 2>/dev/null
end

complete -c hs -n "__hs_use_subcommand" -s H -l hs-home -d 'Path to hyper script home' -r -F
complete -k -c hs -n "__hs_use_subcommand" -s s -l select -d 'Select by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags_and_types both)"
complete -c hs -n "__hs_use_subcommand" -l recent -d 'Show scripts within recent days.' -r -f -a ""
complete -c hs -n "__hs_use_subcommand" -l prompt-level -d 'Prompt level of fuzzy finder.' -r -f -a "never always smart on-multi-fuzz"
complete -c hs -n "__hs_use_subcommand" -l toggle -d 'Toggle named selector temporarily' -r -f -a "(__hs_list_named_selectors)"
complete -c hs -n "__hs_use_subcommand" -l no-trace -d 'Do not record history'
complete -c hs -n "__hs_use_subcommand" -l humble -d 'Do not affect script time (but will still record history)'
complete -c hs -n "__hs_use_subcommand" -l no-caution -d 'Run caution scripts without warning'
complete -c hs -n "__hs_use_subcommand" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__hs_use_subcommand" -s a -l all -d 'Shorthand for `-s=all,^remove --timeless`'
complete -c hs -n "__hs_use_subcommand" -l timeless -d 'Show scripts of all time.'
complete -c hs -n "__hs_use_subcommand" -s h -l help -d 'Prints help information'
complete -c hs -n "__hs_use_subcommand" -s V -l version -d 'Print version information'

complete -c hs -n "__hs_use_subcommand" -f -a "(__hs_list_alias)"

complete -c hs -n "__hs_use_subcommand" -f -a "help" -d 'Prints this message, the help of the given subcommand(s), or a script\'s help message.'
complete -c hs -n "__hs_use_subcommand" -f -a "edit" -d 'Edit hyper script'
complete -c hs -n "__hs_use_subcommand" -f -a "alias" -d 'Manage alias'
# complete -c hs -n "__hs_use_subcommand" -f -a "run" -d 'Run the script' # very rarely needed
complete -c hs -n "__hs_use_subcommand" -f -a "which" -d 'Execute the script query and get the exact file'
complete -c hs -n "__hs_use_subcommand" -f -a "cat" -d 'Print the script to standard output'
complete -c hs -n "__hs_use_subcommand" -f -a "rm" -d 'Remove the script'
complete -c hs -n "__hs_use_subcommand" -f -a "ls" -d 'List hyper scripts'
complete -c hs -n "__hs_use_subcommand" -f -a "recent" -d 'Set recent filter'
complete -c hs -n "__fish_seen_subcommand_from recent" -f -a "timeless no-neglect"
complete -c hs -n "__hs_use_subcommand" -f -a "cp" -d 'Copy the script to another one'
complete -c hs -n "__hs_use_subcommand" -f -a "mv" -d 'Move the script to another one'
complete -c hs -n "__hs_use_subcommand" -f -a "types" -d 'Manage script types'
complete -c hs -n "__hs_use_subcommand" -f -a "tags" -d 'Manage script tags. If a tag selector is given, store it to config, otherwise show tag information.'
complete -c hs -n "__hs_use_subcommand" -f -a "history" -d 'Manage script history'

complete -k -c hs -n "__fish_seen_subcommand_from help" -s s -l select -d 'Select by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags_and_types both)"
complete -c hs -n "__fish_seen_subcommand_from help" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from help" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from help" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from help" -l humble -d 'Do not affect script time (but will still record history)'
complete -c hs -n "__fish_seen_subcommand_from help" -l no-caution -d 'Run caution scripts without warning'
complete -c hs -n "__fish_seen_subcommand_from help" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from help" -s a -l all -d 'Shorthand for `-s=all,^remove --timeless`'
complete -c hs -n "__fish_seen_subcommand_from help" -l timeless -d 'Show scripts of all time.'

complete -c hs -n "__fish_seen_subcommand_from edit" -s T -l ty -d 'Type of the script, e.g. `sh`' -r -f -a "(__hs_list_types)"
complete -k -c hs -n "__fish_seen_subcommand_from edit" -s t -l tags -r -f -a "(__hs_list_tags both)"
complete -k -c hs -n "__fish_seen_subcommand_from edit" -s s -l select -d 'Select by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags_and_types both)"
complete -c hs -n "__fish_seen_subcommand_from edit" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from edit" -s n -l no-template
complete -c hs -n "__fish_seen_subcommand_from edit" -l fast -d 'Create script without invoking the editor'
complete -c hs -n "__fish_seen_subcommand_from edit" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from edit" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from edit" -l humble -d 'Do not affect script time (but will still record history)'
complete -c hs -n "__fish_seen_subcommand_from edit" -l no-caution -d 'Run caution scripts without warning'
complete -c hs -n "__fish_seen_subcommand_from edit" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from edit" -s a -l all -d 'Shorthand for `-s=all,^remove --timeless`'
complete -c hs -n "__fish_seen_subcommand_from edit" -l timeless -d 'Show scripts of all time.'

complete -k -c hs -n "__fish_seen_subcommand_from alias" -s s -l select -d 'Select by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags_and_types both)"
complete -c hs -n "__fish_seen_subcommand_from alias" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from alias" -s u -l unset -d 'Unset an alias.'
complete -c hs -n "__fish_seen_subcommand_from alias" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from alias" -l humble -d 'Do not affect script time (but will still record history)'
complete -c hs -n "__fish_seen_subcommand_from alias" -l no-caution -d 'Run caution scripts without warning'
complete -c hs -n "__fish_seen_subcommand_from alias" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from alias" -s a -l all -d 'Shorthand for `-s=all,^remove --timeless`'
complete -c hs -n "__fish_seen_subcommand_from alias" -l timeless -d 'Show scripts of all time.'
complete -c hs -n "__fish_seen_subcommand_from run" -s r -l repeat
complete -k -c hs -n "__fish_seen_subcommand_from run" -s s -l select -d 'Select by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags_and_types both)"
complete -c hs -n "__fish_seen_subcommand_from run" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from run" -l dummy -d 'Add a dummy run history instead of actually running it'
complete -c hs -n "__fish_seen_subcommand_from run" -s p -l previous
complete -c hs -n "__fish_seen_subcommand_from run" -s d -l dir
complete -c hs -n "__fish_seen_subcommand_from run" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from run" -l humble -d 'Do not affect script time (but will still record history)'
complete -c hs -n "__fish_seen_subcommand_from run" -l no-caution -d 'Run caution scripts without warning'
complete -c hs -n "__fish_seen_subcommand_from run" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from run" -s a -l all -d 'Shorthand for `-s=all,^remove --timeless`'
complete -c hs -n "__fish_seen_subcommand_from run" -l timeless -d 'Show scripts of all time.'
complete -k -c hs -n "__fish_seen_subcommand_from which" -s s -l select -d 'Select by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags_and_types both)"
complete -c hs -n "__fish_seen_subcommand_from which" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from which" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from which" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from which" -l humble -d 'Do not affect script time (but will still record history)'
complete -c hs -n "__fish_seen_subcommand_from which" -l no-caution -d 'Run caution scripts without warning'
complete -c hs -n "__fish_seen_subcommand_from which" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from which" -s a -l all -d 'Shorthand for `-s=all,^remove --timeless`'
complete -c hs -n "__fish_seen_subcommand_from which" -l timeless -d 'Show scripts of all time.'
complete -k -c hs -n "__fish_seen_subcommand_from cat" -s s -l select -d 'Select by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags_and_types both)"
complete -c hs -n "__fish_seen_subcommand_from cat" -l with -d 'Read with other program, e.g. bat'
complete -c hs -n "__fish_seen_subcommand_from cat" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from cat" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from cat" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from cat" -l humble -d 'Do not affect script time (but will still record history)'
complete -c hs -n "__fish_seen_subcommand_from cat" -l no-caution -d 'Run caution scripts without warning'
complete -c hs -n "__fish_seen_subcommand_from cat" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from cat" -s a -l all -d 'Shorthand for `-s=all,^remove --timeless`'
complete -c hs -n "__fish_seen_subcommand_from cat" -l timeless -d 'Show scripts of all time.'

complete -k -c hs -n "__fish_seen_subcommand_from rm" -s s -l select -d 'Select by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags_and_types both)"
complete -c hs -n "__fish_seen_subcommand_from rm" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from rm" -l purge -d 'Actually remove scripts, rather than hiding them with tag.'
complete -c hs -n "__fish_seen_subcommand_from rm" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from rm" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from rm" -l humble -d 'Do not affect script time (but will still record history)'
complete -c hs -n "__fish_seen_subcommand_from rm" -l no-caution -d 'Run caution scripts without warning'
complete -c hs -n "__fish_seen_subcommand_from rm" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from rm" -s a -l all -d 'Shorthand for `-s=all,^remove --timeless`'
complete -c hs -n "__fish_seen_subcommand_from rm" -l timeless -d 'Show scripts of all time.'

complete -c hs -n "__fish_prev_arg_in types" -f -a "template"
complete -c hs -n "__fish_seen_subcommand_from types" -f -a "(__hs_list_types)"
complete -c hs -n "__fish_seen_subcommand_from template" -s e -l edit

complete -c hs -n "__fish_seen_subcommand_from ls" -l grouping -d 'Grouping style.' -r -f -a "tag tree none"
complete -k -c hs -n "__fish_seen_subcommand_from ls" -s s -l select -d 'Select by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags_and_types both)"
complete -c hs -n "__fish_seen_subcommand_from ls" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from ls" -s l -l long -d 'Show verbose information.'
complete -c hs -n "__fish_seen_subcommand_from ls" -l plain -d 'No color and other decoration.'
complete -c hs -n "__fish_seen_subcommand_from ls" -l limit -d 'Limit the amount of scripts found.'
complete -c hs -n "__fish_seen_subcommand_from ls" -l file -d 'Show file path to the script.'
complete -c hs -n "__fish_seen_subcommand_from ls" -l name -d 'Show name of the script.'
complete -c hs -n "__fish_seen_subcommand_from ls" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from ls" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from ls" -l humble -d 'Do not affect script time (but will still record history)'
complete -c hs -n "__fish_seen_subcommand_from ls" -l no-caution -d 'Run caution scripts without warning'
complete -c hs -n "__fish_seen_subcommand_from ls" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from ls" -s a -l all -d 'Shorthand for `-s=all,^remove --timeless`'
complete -c hs -n "__fish_seen_subcommand_from ls" -l timeless -d 'Show scripts of all time.'
complete -k -c hs -n "__fish_seen_subcommand_from cp" -s t -l tags -r -f -a "(__hs_list_tags both)"
complete -k -c hs -n "__fish_seen_subcommand_from cp" -s s -l select -d 'Select by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags_and_types both)"
complete -c hs -n "__fish_seen_subcommand_from cp" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from cp" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from cp" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from cp" -l humble -d 'Do not affect script time (but will still record history)'
complete -c hs -n "__fish_seen_subcommand_from cp" -l no-caution -d 'Run caution scripts without warning'
complete -c hs -n "__fish_seen_subcommand_from cp" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from cp" -s a -l all -d 'Shorthand for `-s=all,^remove --timeless`'
complete -c hs -n "__fish_seen_subcommand_from cp" -l timeless -d 'Show scripts of all time.'
complete -c hs -n "__fish_seen_subcommand_from mv" -s T -l ty -d 'Type of the script, e.g. `sh`' -r -f -a "(__hs_list_types --no-sub)"
complete -k -c hs -n "__fish_seen_subcommand_from mv" -s t -l tags -r -f -a "(__hs_list_tags both)"
complete -k -c hs -n "__fish_seen_subcommand_from mv" -s s -l select -d 'Select by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags_and_types both)"
complete -c hs -n "__fish_seen_subcommand_from mv" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from mv" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from mv" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from mv" -l humble -d 'Do not affect script time (but will still record history)'
complete -c hs -n "__fish_seen_subcommand_from mv" -l no-caution -d 'Run caution scripts without warning'
complete -c hs -n "__fish_seen_subcommand_from mv" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from mv" -s a -l all -d 'Shorthand for `-s=all,^remove --timeless`'
complete -c hs -n "__fish_seen_subcommand_from mv" -l timeless -d 'Show scripts of all time.'

complete -k -c hs -n "__fish_seen_subcommand_from tags" -s s -l select -d 'Select by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags_and_types both)"
complete -c hs -n "__fish_seen_subcommand_from tags" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from tags" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from tags" -l humble -d 'Do not affect script time (but will still record history)'
complete -c hs -n "__fish_seen_subcommand_from tags" -l no-caution -d 'Run caution scripts without warning'
complete -c hs -n "__fish_seen_subcommand_from tags" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from tags" -s a -l all -d 'Shorthand for `-s=all,^remove --timeless`'
complete -c hs -n "__fish_seen_subcommand_from tags" -l timeless -d 'Show scripts of all time.'
complete -c hs -n "__fish_prev_arg_in tags" -f -a "unset"
complete -c hs -n "__fish_prev_arg_in tags" -f -a "set"
complete -c hs -n "__fish_prev_arg_in tags" -f -a "ls"
complete -c hs -n "__fish_prev_arg_in tags" -f -a "toggle"
complete -c hs -n "__fish_seen_subcommand_from tags" -s n -l name -r -f -a "(__hs_list_named_selectors)"
complete -c hs -n "__fish_seen_subcommand_from set" -s n -l name -r -f -a "(__hs_list_named_selectors)"
complete -c hs -n "__fish_seen_subcommand_from ls" -s k -l known # FIXME: 這會補到另一個 ls 上 =_=
complete -k -c hs -n "__fish_prev_arg_in tags" -f -a "(__hs_list_tags_and_types append)"
complete -k -c hs -n "__fish_seen_subcommand_from set" -f -a "(__hs_list_tags_and_types append)"
complete -c hs -n "__fish_seen_subcommand_from unset" -f -a "(__hs_list_named_selectors)"
complete -c hs -n "__fish_seen_subcommand_from toggle" -f -a "(__hs_list_named_selectors)"

complete -k -c hs -n "__fish_prev_arg_in alias" -f -a "(__hs_list_alias)"

complete -k -c hs -n "__fish_seen_subcommand_from history" -s s -l select -d 'Select by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags_and_types both)"
complete -c hs -n "__fish_seen_subcommand_from history" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from history" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from history" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from history" -l humble -d 'Do not affect script time (but will still record history)'
complete -c hs -n "__fish_seen_subcommand_from history" -l no-caution -d 'Run caution scripts without warning'
complete -c hs -n "__fish_seen_subcommand_from history" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from history" -s a -l all -d 'Shorthand for `-s=all,^remove --timeless`'
complete -c hs -n "__fish_prev_arg_in history" -l timeless -d 'Show scripts of all time.'
complete -c hs -n "__fish_prev_arg_in history" -f -a "rm"
complete -c hs -n "__fish_prev_arg_in history" -f -a "rm-id" -d 'Remove an event by it\'s id.
Useful if you want to keep those illegal arguments from polluting the history.'
complete -c hs -n "__fish_prev_arg_in history" -f -a "humble" -d 'Humble an event by it\'s id.'
complete -c hs -n "__fish_prev_arg_in history" -f -a "show"
complete -c hs -n "__fish_prev_arg_in history" -f -a "neglect"
complete -c hs -n "__fish_prev_arg_in history" -f -a "amend"
complete -c hs -n "__fish_prev_arg_in history" -f -a "tidy"
complete -k -c hs -n "__fish_seen_subcommand_from rm" -s s -l select -d 'Select by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags_and_types both)"
complete -c hs -n "__fish_seen_subcommand_from rm" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from rm" -l display -r -f -a "all env args"
complete -c hs -n "__fish_seen_subcommand_from rm" -s d -l dir
complete -c hs -n "__fish_seen_subcommand_from rm" -s d -l no-humble
complete -c hs -n "__fish_seen_subcommand_from rm" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from rm" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from rm" -l humble -d 'Do not affect script time (but will still record history)'
complete -c hs -n "__fish_seen_subcommand_from rm" -l no-caution -d 'Run caution scripts without warning'
complete -c hs -n "__fish_seen_subcommand_from rm" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from rm" -s a -l all -d 'Shorthand for `-s=all,^remove --timeless`'
complete -c hs -n "__fish_seen_subcommand_from rm" -l timeless -d 'Show scripts of all time.'
complete -k -c hs -n "__fish_seen_subcommand_from rm-id" -s s -l select -d 'Select by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags_and_types both)"
complete -c hs -n "__fish_seen_subcommand_from rm-id" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from rm-id" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from rm-id" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from rm-id" -l humble -d 'Do not affect script time (but will still record history)'
complete -c hs -n "__fish_seen_subcommand_from rm-id" -l no-caution -d 'Run caution scripts without warning'
complete -c hs -n "__fish_seen_subcommand_from rm-id" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from rm-id" -s a -l all -d 'Shorthand for `-s=all,^remove --timeless`'
complete -c hs -n "__fish_seen_subcommand_from rm-id" -l timeless -d 'Show scripts of all time.'
complete -c hs -n "__fish_seen_subcommand_from show" -s l -l limit
complete -c hs -n "__fish_seen_subcommand_from show" -s o -l offset
complete -c hs -n "__fish_seen_subcommand_from show" -s d -l dir
complete -k -c hs -n "__fish_seen_subcommand_from show" -s s -l select -d 'Select by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags_and_types both)"
complete -c hs -n "__fish_seen_subcommand_from show" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from show" -l with-name
complete -c hs -n "__fish_seen_subcommand_from show" -l display -r -f -a "all env args"
complete -c hs -n "__fish_seen_subcommand_from show" -l no-humble
complete -c hs -n "__fish_seen_subcommand_from show" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from show" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from show" -l humble -d 'Do not affect script time (but will still record history)'
complete -c hs -n "__fish_seen_subcommand_from show" -l no-caution -d 'Run caution scripts without warning'
complete -c hs -n "__fish_seen_subcommand_from show" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from show" -s a -l all -d 'Shorthand for `-s=all,^remove --timeless`'
complete -c hs -n "__fish_seen_subcommand_from show" -l timeless -d 'Show scripts of all time.'
complete -k -c hs -n "__fish_seen_subcommand_from neglect" -s s -l select -d 'Select by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags_and_types both)"
complete -c hs -n "__fish_seen_subcommand_from neglect" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from neglect" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from neglect" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from neglect" -l humble -d 'Do not affect script time (but will still record history)'
complete -c hs -n "__fish_seen_subcommand_from neglect" -l no-caution -d 'Run caution scripts without warning'
complete -c hs -n "__fish_seen_subcommand_from neglect" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from neglect" -s a -l all -d 'Shorthand for `-s=all,^remove --timeless`'
complete -c hs -n "__fish_seen_subcommand_from neglect" -l timeless -d 'Show scripts of all time.'
complete -k -c hs -n "__fish_seen_subcommand_from amend" -s s -l select -d 'Select by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags_and_types both)"
complete -c hs -n "__fish_seen_subcommand_from amend" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from amend" -s e -l env
complete -c hs -n "__fish_seen_subcommand_from amend" -l no-env
complete -c hs -n "__fish_seen_subcommand_from amend" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from amend" -l humble -d 'Do not affect script time (but will still record history)'
complete -c hs -n "__fish_seen_subcommand_from amend" -l no-caution -d 'Run caution scripts without warning'
complete -c hs -n "__fish_seen_subcommand_from amend" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from amend" -s a -l all -d 'Shorthand for `-s=all,^remove --timeless`'
complete -c hs -n "__fish_seen_subcommand_from amend" -l timeless -d 'Show scripts of all time.'
complete -k -c hs -n "__fish_seen_subcommand_from tidy" -s s -l select -d 'Select by tags, e.g. `all,^mytag`' -r -f -a "(__hs_list_tags_and_types both)"
complete -c hs -n "__fish_seen_subcommand_from tidy" -l recent -d 'Show scripts within recent days.'
complete -c hs -n "__fish_seen_subcommand_from tidy" -s h -l help -d 'Prints help information'
complete -c hs -n "__fish_seen_subcommand_from tidy" -l no-trace -d 'Do not record history'
complete -c hs -n "__fish_seen_subcommand_from tidy" -l humble -d 'Do not affect script time (but will still record history)'
complete -c hs -n "__fish_seen_subcommand_from tidy" -l no-caution -d 'Run caution scripts without warning'
complete -c hs -n "__fish_seen_subcommand_from tidy" -s A -l archaeology -d 'Show scripts NOT within recent days'
complete -c hs -n "__fish_seen_subcommand_from tidy" -s a -l all -d 'Shorthand for `-s=all,^remove --timeless`'
complete -c hs -n "__fish_seen_subcommand_from tidy" -l timeless -d 'Show scripts of all time.'
