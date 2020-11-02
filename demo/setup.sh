#! /usr/bin/env bash

set -e

cd $(dirname $0)

BACK=~/.config/hyper_scripter_backup
HS_HOME=~/.config/hyper_scripter

if [ -d $BACK ]; then
    echo "已存在備份資料夾，不動作"
    exit 1
fi

mv $HS_HOME $BACK
cp ./scripts $HS_HOME -r
