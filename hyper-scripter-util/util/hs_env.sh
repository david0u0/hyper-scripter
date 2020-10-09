cd $(dirname $0)

while true; do
    CUR=$(pwd)
    if [ -f ".script_info.db" ]; then
        break
    fi
    if [ "$CUR" = "/" ]; then
        echo "Fail to find hyper script path" 1>&2
        exit 1
    fi
    cd ..
done

EXE=$(cat .hs_exe_path)

if [ $? != 0 ]; then
    echo "Fail to locate hyper script executable" 1>&2
    exit 1
fi

echo $CUR:$EXE