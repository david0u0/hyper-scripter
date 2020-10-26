# [HS_USAGE]: Open hyper scripter home. If a script is given, it will be opened on a splitted editor.
# [HS_USAGE]:
# [HS_USAGE]: USAGE:
# [HS_USAGE]:     hs open-hs [script query]
# [HS_USAGE]:     OR
# [HS_USAGE]:     hs open-hs ! # open `.config.toml`

set -e

DIR=$(dirname $0)
ENV=$(bash $DIR/hs_env.sh)
NAME=open-hs

IFS=: read HS_HOME HS_EXE <<< $ENV

cd $HS_HOME

if [ -z $1 ]; then
    FILE=
else
    if [ "$1" == "!" ]; then
        FILE='.config.toml'
    else
        FILE=$($HS_EXE -H $HS_HOME which $1)
    fi
fi

set -- $(stty size)
tmux new-session -s $NAME -d -x "$2" -y "$(($1 - 1))"
if [ ! -z "$FILE" ]; then
    tmux split-window -h "vim $FILE; $SHELL"
fi
tmux -2 attach-session -d
