if [[ -v ONLY_TOUCH ]]; then
    FILES=""
    for arg in "$@"
    do
        IFS=';' read -ra ARR <<< "$ONLY_TOUCH"
        for i in "${ARR[@]}"; do
            base=$(basename $arg)
            if [[ $base =~ "$i" ]]; then
                FILES="$FILES $arg"
            fi
        done
    done

    if [ ! "$FILES" == "" ]; then
        sleep 0.01
        touch $FILES
    fi
else
    sleep 0.01
    touch $@
fi
