#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
project="$root/src/RustyRoguelike.Product/RustyRoguelike.Product.csproj"
runtime="$root/.runtime/runtime-pack-cabba0f"
staged_product="$root/src/RustyRoguelike.Product/obj/Rusty.Engine/Product"
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

test -x "$runtime/bin/rusty-product-host"
dotnet msbuild "$project" -t:StageRustyEngineCoreClrProduct
"$runtime/bin/rusty-product-host" \
  --product "$staged_product" \
  --loader coreclr \
  --persistence-root "$persistence_root" \
  >"$host_log" 2>&1 &
host_pid=$!

origin=
for _ in {1..100}; do
  if ! kill -0 "$host_pid" 2>/dev/null; then
    sed -n '1,200p' "$host_log" >&2
    exit 1
  fi
  origin=$(sed -n 's/^C# .* product host listening at //p' "$host_log" | tail -1)
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
  local active=$4
  local payload
  payload=$(jq -cn --argjson runtime "$runtime" --arg sequence "$sequence" --arg intent "$intent" --argjson active "$active" \
    '{batch: [{runtime: $runtime, sequence: $sequence, context: "gameplay.default", intent: $intent, value: {kind: "digital", active: $active}}]}')
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

latest_session_projection() {
  local output_file=$1
  local status
  set +e
  curl --no-buffer --silent --max-time 1 \
    -H 'Accept: text/event-stream' \
    "$origin/__rusty/product/runtime/outputs" >"$output_file"
  status=$?
  set -e
  [[ "$status" == 0 || "$status" == 28 ]] || return "$status"
  sed -n 's/^data: //p' "$output_file" \
    | jq -cs '[.[] | select(.kind == "ui-projection" and .envelope.stream == "rusty-roguelike.session")] | last'
}

start=$(post_lifecycle start null)
jq -e '.accepted == true and .operation == "start" and .readout.state == "running"' <<<"$start" >/dev/null
runtime=$(jq -c '.binding' <<<"$start")

inactive_begin=$(post_direct_input "$runtime" 1 roguelike.begin false)
jq -e '.accepted == true' <<<"$inactive_begin" >/dev/null
inactive_begin_step=$(admit_demand_step)
jq -e '.accepted == true' <<<"$inactive_begin_step" >/dev/null

save_before_begin=$(post_direct_input "$runtime" 2 roguelike.save true)
jq -e '.accepted == true' <<<"$save_before_begin" >/dev/null
save_before_begin_step=$(admit_demand_step)
jq -e '.accepted == true' <<<"$save_before_begin_step" >/dev/null
save_file="$persistence_root/rusty-roguelike/starter-session"
test -f "$save_file"
grep -aq '"revision":0' "$save_file"
save_before_inactive=$(sha256sum "$save_file" | awk '{print $1}')

inactive_save=$(post_direct_input "$runtime" 3 roguelike.save false)
jq -e '.accepted == true' <<<"$inactive_save" >/dev/null
inactive_save_step=$(admit_demand_step)
jq -e '.accepted == true' <<<"$inactive_save_step" >/dev/null
test "$save_before_inactive" = "$(sha256sum "$save_file" | awk '{print $1}')"

begin=$(post_direct_input "$runtime" 4 roguelike.begin true)
jq -e '.accepted == true' <<<"$begin" >/dev/null
begin_step=$(admit_demand_step)
jq -e '.accepted == true' <<<"$begin_step" >/dev/null

move=$(post_direct_input "$runtime" 5 roguelike.move.east true)
jq -e '.accepted == true' <<<"$move" >/dev/null
move_step=$(admit_demand_step)
jq -e '.accepted == true' <<<"$move_step" >/dev/null
session_after_move=$(latest_session_projection "$run_dir/after-move.sse")
jq -e '
  .envelope.contract == "rusty-roguelike.session.v1"
  and (.envelope.value.revision | tonumber) >= 2
  and (.envelope.value.activationIndex | tonumber) >= 1
  and .envelope.value.currentActor == "mira"
  and (.envelope.value.partyCellX | tonumber) == 18
  and (.envelope.value.partyCellY | tonumber) == 16
  and .envelope.value.latestReceipt == "none"' <<<"$session_after_move" >/dev/null

wait=$(post_direct_input "$runtime" 6 roguelike.wait true)
jq -e '.accepted == true' <<<"$wait" >/dev/null
wait_step=$(admit_demand_step)
jq -e '.accepted == true' <<<"$wait_step" >/dev/null
session_after_wait=$(latest_session_projection "$run_dir/after-wait.sse")
jq -e '
  (.envelope.value.revision | tonumber) >= 3
  and .envelope.value.currentActor == "brann"
  and (.envelope.value.partyCellX | tonumber) == 18
  and (.envelope.value.partyCellY | tonumber) == 16' <<<"$session_after_wait" >/dev/null
saved_session_revision=$(jq -r '.envelope.value.revision | tonumber | floor' <<<"$session_after_wait")
saved_activation_index=$(jq -r '.envelope.value.activationIndex | tonumber | floor' <<<"$session_after_wait")
saved_session_value=$(jq -c '.envelope.value' <<<"$session_after_wait")

save=$(post_direct_input "$runtime" 7 roguelike.save true)
jq -e '.accepted == true' <<<"$save" >/dev/null
save_step=$(admit_demand_step)
jq -e '.accepted == true' <<<"$save_step" >/dev/null
grep -aq "\"revision\":$saved_session_revision" "$save_file"
grep -aq "\"activationIndex\":$saved_activation_index" "$save_file"

perturb=$(post_direct_input "$runtime" 8 roguelike.wait true)
jq -e '.accepted == true' <<<"$perturb" >/dev/null
perturb_step=$(admit_demand_step)
jq -e '.accepted == true' <<<"$perturb_step" >/dev/null
session_after_perturb=$(latest_session_projection "$run_dir/after-perturb.sse")
jq -e --argjson saved "$saved_session_revision" '(.envelope.value.revision | tonumber) > $saved' <<<"$session_after_perturb" >/dev/null

load=$(post_direct_input "$runtime" 9 roguelike.load true)
jq -e '.accepted == true' <<<"$load" >/dev/null
load_step=$(admit_demand_step)
jq -e '.accepted == true' <<<"$load_step" >/dev/null
session_after_load=$(latest_session_projection "$run_dir/after-load.sse")
jq -e --argjson revision "$saved_session_revision" --argjson activation "$saved_activation_index" '
  (.envelope.value.revision | tonumber) == $revision
  and (.envelope.value.activationIndex | tonumber) == $activation' <<<"$session_after_load" >/dev/null
jq -e --argjson saved "$saved_session_value" '.envelope.value == $saved' <<<"$session_after_load" >/dev/null

pause=$(post_lifecycle pause "$runtime")
jq -e '.accepted == true and .readout.state == "paused"' <<<"$pause" >/dev/null
runtime=$(jq -c '.binding' <<<"$pause")

resume=$(post_lifecycle resume "$runtime")
jq -e '.accepted == true and .readout.state == "running"' <<<"$resume" >/dev/null
runtime=$(jq -c '.binding' <<<"$resume")

restart=$(post_lifecycle restart "$runtime")
jq -e '.accepted == true and .readout.state == "running"' <<<"$restart" >/dev/null
runtime=$(jq -c '.binding' <<<"$restart")

curl --fail --silent --show-error "$origin/product-ui/main.js" | grep -Fq 'Rusty Roguelike'

shutdown=$(post_lifecycle shutdown "$runtime")
jq -e '.accepted == true and .readout.state == "shutdown"' <<<"$shutdown" >/dev/null

echo "CoreCLR packaged product lifecycle and loopback host exercise passed"
