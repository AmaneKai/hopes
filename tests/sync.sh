#!/bin/sh
set -eu

bin=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
root=$(mktemp -d)
sock=hopes-sync-$$

case $(uname) in
Darwin) export LANG=en_US.UTF-8 ;;
*) export LANG=C.UTF-8 ;;
esac
export TERM=xterm-256color GIT_TERMINAL_PROMPT=0
export GIT_AUTHOR_NAME=ci GIT_AUTHOR_EMAIL=ci@example.com
export GIT_COMMITTER_NAME=ci GIT_COMMITTER_EMAIL=ci@example.com

cleanup() {
    tmux -L "$sock" kill-server 2> /dev/null || true
    rm -rf "$root"
}
trap cleanup EXIT

screen() {
    tmux -L "$sock" capture-pane -p -t "$1"
}

fail() {
    echo "FAIL: $*" >&2
    for pane in a b; do
        echo "--- pane $pane" >&2
        screen "$pane" >&2 2> /dev/null || true
    done
    exit 1
}

wait_screen() {
    tries=0
    while [ "$tries" -lt $(($3 * 10)) ]; do
        screen "$1" | grep -q "$2" && return 0
        sleep 0.1
        tries=$((tries + 1))
    done
    fail "pane $1 never showed '$2'"
}

wait_remote() {
    tries=0
    while [ "$tries" -lt $(($2 * 10)) ]; do
        git -C "$root/remote.git" show main:items.json 2> /dev/null | grep -q "$1" && return 0
        sleep 0.1
        tries=$((tries + 1))
    done
    fail "remote never received '$1'"
}

start() {
    tmux -u -L "$sock" new-session -d -s "$1" -x 110 -y 24 "HOPES_CONFIG=$root/cfg$2.toml $bin"
}

add_task() {
    tmux -L "$sock" send-keys -t "$1" a
    tmux -L "$sock" send-keys -t "$1" -l "$2"
    tmux -L "$sock" send-keys -t "$1" Enter
}

git init -q --bare -b main "$root/remote.git"
git clone -q "$root/remote.git" "$root/dataA" 2> /dev/null
(cd "$root/dataA" && echo '[]' > items.json && git add items.json && git commit -qm init && git push -q origin main)
git clone -q "$root/remote.git" "$root/dataB" 2> /dev/null
for d in A B; do
    printf '[sync]\ndebounce_seconds = 1\ndata_dir = "%s/data%s"\n' "$root" "$d" > "$root/cfg$d.toml"
done

start a A
wait_screen a "Synced" 15
add_task a "from A"
wait_remote "from A" 20

start b B
wait_screen b "from A" 20
wait_screen b "Pulled remote changes" 5

add_task b "from B"
sleep 0.3
tmux -L "$sock" send-keys -t b q
wait_remote "from B" 20

tmux -L "$sock" send-keys -t a S
wait_screen a "from B" 20

add_task a "雨音 unicode"
wait_remote "雨音 unicode" 20

echo "sync ok"
