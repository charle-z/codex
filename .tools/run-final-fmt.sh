#!/bin/sh
set -eu
export CARGO_HOME=/workspace/.tools/cargo-home
export RUSTUP_HOME=/workspace/.tools/rustup-home
export PATH=/workspace/.tools/cargo-home/bin:/workspace/.tools:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
export CARGO_BUILD_JOBS=1
cd /workspace/codex-rs
exec /workspace/.tools/just fmt
