DIR=$(dirname $0)
HS_PATH=$(bash $DIR/hs_path.sh)

if [ $? != 0 ]; then
    echo Fail finding hyper script path
    exit 1
fi

for script in $(hs --timeless -p $HS_PATH -f deleted ls --plain --no-grouping --name); do
    echo purge $script !
    hs -p $HS_PATH -t deleted rm --purge =$script
done
