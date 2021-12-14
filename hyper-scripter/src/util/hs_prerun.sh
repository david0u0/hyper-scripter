set -e

>&2 echo running $NAME "$@"

YELLOW='\033[1;33m'
RED='\033[1;31m'
BLUE='\033[1;34m'
CYAN='\033[1;36m'
GREEN='\033[1;32m'
WHITE='\033[1;37m'
NC='\033[0m'

echo "$HS_ENV_HELP" | while read line
do
    if [ "$line" = "" ]; then
        continue
    fi

    ENV=$(cut -d ' ' -f 1 <<< "$line")
    MSG=$(cut -d ' ' -f 2- <<< "$line ")
    MSG=$(echo $MSG | tr -s " ")
    if [[ -v $ENV ]]; then
        VAR=$RED${!ENV}$NC
    else
        VAR=$WHITE--$NC
    fi
    if [ ! "$MSG" = "" ]; then
        MSG=" ($MSG)"
    fi
    >&2 echo -e ${CYAN}$ENV:${NC} $VAR$MSG
done
