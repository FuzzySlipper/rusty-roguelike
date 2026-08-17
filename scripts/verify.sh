#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

pnpm run verify:boundaries
# Gameplay authoring drift gate: rebuild the materialized package and fail on
# any diff against the committed artifact (Den 7062).
pnpm run gameplay:check
pnpm run verify:rust
pnpm run verify:native
pnpm run verify:ui
pnpm run verify:build
pnpm run verify:browser
