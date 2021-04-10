set -e

if [ "$1" == "home" ]; then
    echo $HS_HOME
elif [ "$2" == "exe" ]; then
    echo $HS_EXE
else
    echo $HS_HOME:$HS_EXE
fi
