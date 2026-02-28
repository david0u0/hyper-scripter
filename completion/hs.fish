function __do_completion
    set cmd (commandline -c)
    set cmd_arr (string split ' ' $cmd)
    if [ -z "$cmd_arr[-1]" ]
        # preserve the last white space
        echo completion-subsystem fish $cmd "''" | xargs hs
    else
        echo completion-subsystem fish $cmd | xargs hs
    end
end

complete -k -c hs -x -a "(__do_completion)"
