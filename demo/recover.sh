#! /usr/bin/env bash

set -e

cd $(dirname $0)

BACK=~/.config/hyper_scripter_backup
HS_HOME=~/.config/hyper_scripter

DEMO_INDICATE=.remove_me_Im_for_demo

if [ -d $HS_HOME ]; then
    if [ ! -f $HS_HOME/$DEMO_INDICATE ]; then
        echo "當前的腳本之家並非展示用！"
        exit 1
    fi
fi

rm $HS_HOME -rf
mv $BACK $HS_HOME