# Hello, scripter! Here are some useful commands to begin with:

export NAME="test"
export VAR="${VAR:-default}"
cd ~/Workspace/hyper-scripter/hyper-scripter

echo 我在 $(realpath $(dirname $0))
