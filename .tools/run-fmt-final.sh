#!/bin/sh
set -eu
export CARGO_HOME=/workspace/.tools/cargo-home
export RUSTUP_HOME=/workspace/.tools/rustup-home
export PATH=/workspace/.tools/cargo-home/bin:/workspace/.tools:/usr/local/bin:/usr/bin:/bin
cd /workspace/codex-rs
/workspace/.tools/just fmt
