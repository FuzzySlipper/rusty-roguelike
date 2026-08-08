#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

proof_output=$(mktemp -t rusty-roguelike-native-proof.XXXXXX.log)
trap 'rm -f "$proof_output"' EXIT

if [[ "$(uname -s)" == "Linux" ]]; then
  xvfb-run -a env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET \
    GDK_BACKEND=x11 LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 \
    cargo run --manifest-path rust/Cargo.toml -p rusty-roguelike \
      --bin rusty-roguelike-native --locked -- --proof >"$proof_output" 2>&1
else
  cargo run --manifest-path rust/Cargo.toml -p rusty-roguelike \
    --bin rusty-roguelike-native --locked -- --proof >"$proof_output" 2>&1
fi

grep -F \
  'RUSTY_ROGUELIKE_NATIVE_PROOF_OK frame=true views=true camera=true resize=true resource_count=1 input=true pick=true state=true render=true authority_round_trip=true lifecycle=disposed' \
  "$proof_output"
