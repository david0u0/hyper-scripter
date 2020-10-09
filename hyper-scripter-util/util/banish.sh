DIR=$(dirname $0)
HS_PATH=$(bash $DIR/hs_path.sh)

if [ $? != 0 ]; then
    echo Fail to find hyper script path
    exit 1
fi

HS_EXE=$(cat $HS_PATH/.hs_exe_path)
if [ $? != 0 ]; then
    echo Fail to locate hyper script executable
    exit 1
fi

for script in $($HS_EXE --timeless -p $HS_PATH -f deleted ls --plain --no-grouping --name); do
    echo purge $script !
    $HS_EXE -p $HS_PATH -f deleted rm --purge =$script
done
