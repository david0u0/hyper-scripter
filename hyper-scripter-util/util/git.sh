DIR=$(dirname $0)
ENV=$(bash $DIR/hs_env.sh)
if [ $? != 0 ]; then
    exit 1
fi
IFS=: read HS_PATH HS_EXE <<< $ENV

cd $HS_PATH
if [ "$1" = "init" ]; then
    if [ ! -f ".gitignore" ]; then
        echo "creating .gitignore!"
        echo ".script_history.db" > .gitignore
        echo "*.db-*" >> .gitignore
        echo ".hs_exe_path" >> .gitignore
    fi
fi

git $@