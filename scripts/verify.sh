#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

pnpm run verify:boundaries
pnpm run verify:rust
pnpm run verify:ui
pnpm run verify:build
pnpm run verify:browser
