if [[ ! -v NO_TOUCH ]]; then
    sleep 0.01
    touch $@
fi
