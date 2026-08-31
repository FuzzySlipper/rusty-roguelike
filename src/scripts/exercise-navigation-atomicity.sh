#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
engine=${RUSTY_ENGINE_ROOT:-"$root/../rusty-engine"}
project="$root/src/RustyRoguelike.NavigationAtomicityProbe/RustyRoguelike.NavigationAtomicityProbe.csproj"
output="$root/src/RustyRoguelike.NavigationAtomicityProbe/bin/Release/net10.0/linux-x64/publish"
host_root="$root/src/RustyRoguelike.NativeProduct/DevelopmentHost"
run_dir=$(mktemp -d)
host_log="$run_dir/host.log"
host_pid=

cleanup() {
  if [[ -n "$host_pid" ]] && kill -0 "$host_pid" 2>/dev/null; then
    kill "$host_pid"
    wait "$host_pid" || true
  fi
  rm -r "$run_dir"
}
trap cleanup EXIT

dotnet publish "$project" -c Release -r linux-x64
cargo run --manifest-path "$engine/rust/crates/csharp-product-runtime/Cargo.toml" --bin csharp-product-runtime --locked -- \
  --library "$output/RustyRoguelike.NavigationAtomicityProbe.so" \
  --bundle-dir "$host_root/browser" \
  --content-dir "$host_root/content" \
  --persistence-root "$run_dir/persistence" \
  --mode demand \
  --port 0 >"$host_log" 2>&1 &
host_pid=$!

origin=
for _ in {1..100}; do
  if ! kill -0 "$host_pid" 2>/dev/null; then
    sed -n '1,200p' "$host_log" >&2
    exit 1
  fi
  origin=$(sed -n 's/^C# NativeAOT product host listening at //p' "$host_log" | tail -1)
  [[ -n "$origin" ]] && break
  sleep 0.05
done
if [[ -z "$origin" ]]; then
  sed -n '1,200p' "$host_log" >&2
  exit 1
fi

start=$(curl --fail --silent --show-error \
  -H 'Content-Type: application/json' \
  --data '{"runtime":null}' \
  "$origin/__rusty/product/runtime/lifecycle/start")
jq -e '.accepted == true and .readout.state == "running"' <<<"$start" >/dev/null

set +e
curl --no-buffer --silent --max-time 1 \
  -H 'Accept: text/event-stream' \
  "$origin/__rusty/product/runtime/outputs" >"$run_dir/proof.sse"
status=$?
set -e
[[ "$status" == 0 || "$status" == 28 ]]
proof=$(sed -n 's/^data: //p' "$run_dir/proof.sse" \
  | jq -cs '[.[] | select(.kind == "ui-projection" and .envelope.stream == "rusty-roguelike.navigation-atomicity")] | last')
jq -e '
  .envelope.contract == "rusty-roguelike.navigation-atomicity.v1"
  and .envelope.value.accepted == "true"
  and .envelope.value.settlementCode == "command-settlement-failed"
  and .envelope.value.productStateUnchanged == "true"
  and .envelope.value.engineNavigationUnchanged == "true"
  and (.envelope.value.retainedPathLength | tonumber) > 0' <<<"$proof" >/dev/null

echo "NativeAOT navigation atomicity proof passed"
