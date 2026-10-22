#!/usr/bin/env sh
# Smallest build: rebuilds the standard library with std's `optimize_for_size` feature.
# Needs a nightly toolchain with the rust-src component:
#   rustup toolchain install nightly --component rust-src
# Panic messages are kept. Output: target/<host-triple>/release/hopes
set -eu

host=$(rustc +nightly -vV | sed -n 's/^host: //p')
cargo +nightly build --release \
    -Zbuild-std=std,panic_abort \
    -Zbuild-std-features=optimize_for_size \
    --target "$host" "$@"
echo "target/$host/release/hopes"
