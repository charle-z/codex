#!/bin/sh
set -eu
export CARGO_HOME=/workspace/.tools/cargo-home
export RUSTUP_HOME=/workspace/.tools/rustup-home
export PATH=/workspace/.tools/cargo-home/bin:/workspace/.tools:/workspace/.mcp-devbox/tools/bin:/workspace/.mcp-devbox/tools/go/bin:/workspace/.mcp-devbox/tools/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
cd /workspace/codex-rs
exec /workspace/.tools/just test -p codex-tui "$@"
