# Verification

The maintained product checks are deliberately focused:

```bash
dotnet run --project src/RustyRoguelike.Product.Checks/RustyRoguelike.Product.Checks.csproj
dotnet msbuild src/RustyRoguelike.Product/RustyRoguelike.Product.csproj -t:StageRustyEngineCoreClrProduct
./.runtime/runtime-pack-cbf35130d06c/bin/rusty dev --project ./src/RustyRoguelike.Product/RustyRoguelike.Product.csproj --runtime ./.runtime/runtime-pack-cbf35130d06c
dotnet msbuild src/RustyRoguelike.Product/RustyRoguelike.Product.csproj -t:VerifyRustyEngineAot
bash src/scripts/exercise-product.sh
bash src/scripts/exercise-navigation-atomicity.sh
```

The small checks executable covers only the review-critical floor trust/null
boundary, full initiative cursor, defeated-actor pruning, Engine `ExactTrack`
opposition bounds, malformed restore data, and fail-atomic action settlement.
The CoreCLR stage builds the managed product and atomically stages its loose
Product directory. `rusty dev` is the normal edit/run command over the exact
runtime pack. The explicit AOT target stages the matching native module. The
exercise starts the staged CoreCLR product through the packaged host on an
isolated loopback port, loads the committed content,
admits begin and explicit movement/Wait activations followed by demand steps, reads the
actual `rusty-roguelike.session` SSE projection to verify revision, activation,
and the current no-admitted-opposition projection, saves, perturbs, loads, and
verifies the restored observable projection and durable snapshot,
checks pause/resume/restart/shutdown lifecycle facts, reads the bundled page,
and cleans its temporary host/persistence directory.

The navigation atomicity exercise launches a dedicated packaged NativeAOT
probe through the same runtime pack. It creates the production `FloorEngineProjection`,
seeds and reads the reverse of the tested Engine navigation path, performs
movement admission through
`FloorEngineProjection.ProposePartyStep`, injects a visibility failure during
the later candidate-settlement stage, then compares the complete product
checkpoint and actual Engine navigation projection/path readouts before and
after. The probe publishes its result only after both owners remain unchanged.

This is an honest first-slice proof. It is not a replacement for the removed
Rust test suites, Node/Nx checks, generated protocol checks, browser E2E
workflow, or legacy CI. Add a focused C# proof only when changing an owned
behavior; do not reintroduce compatibility gates merely to preserve the old
workflow.
