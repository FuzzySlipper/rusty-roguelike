#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
engine=${RUSTY_ENGINE_ROOT:-"$root/../rusty-engine"}
native_project="$root/src/RustyRoguelike.NativeProduct/RustyRoguelike.NativeProduct.csproj"
native_output="$root/src/RustyRoguelike.NativeProduct/bin/Release/net10.0/linux-x64/publish"
host_root="$root/src/RustyRoguelike.NativeProduct/DevelopmentHost"
run_dir=$(mktemp -d)
host_log="$run_dir/host.log"
persistence_root="$run_dir/persistence"
host_pid=
mkdir -p "$persistence_root"

cleanup() {
  if [[ -n "$host_pid" ]] && kill -0 "$host_pid" 2>/dev/null; then
    kill "$host_pid"
    wait "$host_pid" || true
  fi
  rm -r "$run_dir"
}
trap cleanup EXIT

dotnet publish "$native_project" -c Release -r linux-x64
cargo run --manifest-path "$engine/rust/crates/csharp-product-runtime/Cargo.toml" --locked -- \
  --library "$native_output/RustyRoguelike.NativeProduct.so" \
  --bundle-dir "$host_root/browser" \
  --content-dir "$host_root/content" \
  --persistence-root "$persistence_root" \
  --direct-intent roguelike.begin=digital \
  --direct-intent roguelike.save=digital \
  --direct-intent roguelike.load=digital \
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

post_lifecycle() {
  local operation=$1
  local runtime=$2
  curl --fail --silent --show-error \
    -H 'Content-Type: application/json' \
    --data "{\"runtime\":$runtime}" \
    "$origin/__rusty/product/runtime/lifecycle/$operation"
}

post_direct_input() {
  local runtime=$1
  local sequence=$2
  local intent=$3
  local payload
  payload=$(jq -cn --argjson runtime "$runtime" --arg sequence "$sequence" --arg intent "$intent" \
    '{batch: [{runtime: $runtime, sequence: $sequence, context: "gameplay.default", intent: $intent, value: {kind: "digital", active: true}}]}')
  curl --fail --silent --show-error \
    -H 'Content-Type: application/json' \
    --data "$payload" \
    "$origin/__rusty/product/runtime/input"
}

admit_demand_step() {
  curl --fail --silent --show-error \
    -H 'Content-Type: application/json' \
    --data '{}' \
    "$origin/__rusty/product/runtime/admit-demand-step"
}

start=$(post_lifecycle start null)
jq -e '.accepted == true and .operation == "start" and .readout.state == "running"' <<<"$start" >/dev/null
runtime=$(jq -c '.binding' <<<"$start")

begin=$(post_direct_input "$runtime" 1 roguelike.begin)
jq -e '.accepted == true' <<<"$begin" >/dev/null
begin_step=$(admit_demand_step)
jq -e '.accepted == true' <<<"$begin_step" >/dev/null

save=$(post_direct_input "$runtime" 2 roguelike.save)
jq -e '.accepted == true' <<<"$save" >/dev/null
save_step=$(admit_demand_step)
jq -e '.accepted == true' <<<"$save_step" >/dev/null
test -n "$(find "$persistence_root" -type f -print -quit)"

load=$(post_direct_input "$runtime" 3 roguelike.load)
jq -e '.accepted == true' <<<"$load" >/dev/null
load_step=$(admit_demand_step)
jq -e '.accepted == true' <<<"$load_step" >/dev/null

pause=$(post_lifecycle pause "$runtime")
jq -e '.accepted == true and .readout.state == "paused"' <<<"$pause" >/dev/null
runtime=$(jq -c '.binding' <<<"$pause")

resume=$(post_lifecycle resume "$runtime")
jq -e '.accepted == true and .readout.state == "running"' <<<"$resume" >/dev/null
runtime=$(jq -c '.binding' <<<"$resume")

restart=$(post_lifecycle restart "$runtime")
jq -e '.accepted == true and .readout.state == "running"' <<<"$restart" >/dev/null
runtime=$(jq -c '.binding' <<<"$restart")

curl --fail --silent --show-error "$origin/" | grep -Fq 'Rusty Roguelike NativeAOT product spine.'

shutdown=$(post_lifecycle shutdown "$runtime")
jq -e '.accepted == true and .readout.state == "shutdown"' <<<"$shutdown" >/dev/null

echo "NativeAOT product lifecycle and loopback host exercise passed"
