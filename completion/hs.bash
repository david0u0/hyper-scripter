_hs()
{
    args=${COMP_WORDS[@]:0:$((COMP_CWORD+1))}
    cur="${COMP_WORDS[COMP_CWORD]}"

    if [[ -z "$cur" ]]; then
        COMPREPLY=($( echo completion-subsystem bash $args "''" | xargs hs))
    else
        COMPREPLY=($( echo completion-subsystem bash $args | xargs hs))
    fi
} &&
    complete -F _hs hs

# ex: filetype=sh
