function __do_completion
    set cmd (commandline -c)
    set cmd_arr (string split ' ' $cmd)
    set cur "$cmd_arr[-1]"
    if [ -z "$cur" ]
        # preserve the last white space
        echo completion-subsystem fish $cmd "''" | xargs hs
    else
        echo completion-subsystem fish $cmd | xargs hs
    end

    if [ "$status" != "0" ]
        complete -C "'' $cur"
    end
end

complete -k -c hs -x -a "(__do_completion)"
