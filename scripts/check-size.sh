#!/bin/sh
set -eu

file=$1
budget=$2
size=$(wc -c < "$file" | tr -d ' ')

echo "$file: $size bytes (budget $budget)"
if [ "$size" -gt "$budget" ]; then
    echo "binary grew past its budget" >&2
    exit 1
fi
