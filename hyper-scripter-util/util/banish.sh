DIR=$(dirname $0)

ENV=$(bash $DIR/hs_env.sh)
if [ $? != 0 ]; then
    exit 1
fi
IFS=: read HS_PATH HS_EXE <<< $ENV

for script in $($HS_EXE --timeless -p $HS_PATH -f deleted ls --plain --no-grouping --name); do
    echo purge $script !
    $HS_EXE --timeless -p $HS_PATH -f deleted rm --purge =$script
done
