# Verification

The maintained product checks are deliberately focused:

```bash
dotnet run --project src/RustyRoguelike.Product.Checks/RustyRoguelike.Product.Checks.csproj
dotnet build RustyRoguelike.sln -c Release
dotnet publish src/RustyRoguelike.NativeProduct/RustyRoguelike.NativeProduct.csproj -c Release -r linux-x64
bash src/scripts/exercise-native-product.sh
bash src/scripts/exercise-navigation-atomicity.sh
```

The small checks executable covers only the review-critical floor trust/null
boundary, full initiative cursor, defeated-actor pruning, Engine `ExactTrack`
opposition bounds, malformed restore data, and fail-atomic action settlement.
The build checks the managed projects. Publish checks the NativeAOT shared
library. The exercise starts the actual adjacent Engine C# product runtime on
an isolated loopback port, loads the product library and committed content,
admits begin and explicit movement/Wait activations followed by demand steps, reads the
actual `rusty-roguelike.session` SSE projection to verify revision, activation,
and the current no-admitted-opposition projection, saves, perturbs, loads, and
verifies the restored observable projection and durable snapshot,
checks pause/resume/restart/shutdown lifecycle facts, reads the bundled page,
and cleans its temporary host/persistence directory.

The navigation atomicity exercise launches a dedicated NativeAOT probe through
the same real Engine host. It creates the production `FloorEngineProjection`,
seeds and reads a single-cell retained Engine path that differs from the tested
move, performs movement admission through
`FloorEngineProjection.ProposePartyStep`, injects a visibility failure during
the later candidate-settlement stage, then compares the complete product
checkpoint and actual Engine navigation projection/path readouts before and
after. The probe publishes its result only after both owners remain unchanged.

This is an honest first-slice proof. It is not a replacement for the removed
Rust test suites, Node/Nx checks, generated protocol checks, browser E2E
workflow, or legacy CI. Add a focused C# proof only when changing an owned
behavior; do not reintroduce compatibility gates merely to preserve the old
workflow.
