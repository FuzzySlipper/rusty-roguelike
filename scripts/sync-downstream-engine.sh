#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
manifest="$repo_root/rust/Cargo.toml"

cargo update --manifest-path "$manifest" -p rusty-engine
python3 "$repo_root/scripts/check-downstream-engine-freshness.py" \
  --manifest "$manifest" "$repo_root/rust/Cargo.lock"
