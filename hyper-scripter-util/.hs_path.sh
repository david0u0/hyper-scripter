while true; do
    CUR=$(pwd)
    if [ -f ".script_info.db" ]; then
        echo $CUR
        exit 0
    fi
    if [ "$CUR" == "/" ]; then
        exit 1
    fi
    cd ..
done