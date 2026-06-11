#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
src_tauri="$repo_root/src-tauri"
bin_dir="$src_tauri/bin"
triple="$(rustc --print host-tuple)"

cd "$src_tauri"
cargo build --release -p calliop-llm-worker
mkdir -p "$bin_dir"
cp "target/release/calliop-llm-worker" "$bin_dir/calliop-llm-worker-$triple"
