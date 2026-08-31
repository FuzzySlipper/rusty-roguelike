# Verification

The maintained product checks are deliberately focused:

```bash
dotnet run --project src/RustyRoguelike.Product.Checks/RustyRoguelike.Product.Checks.csproj
dotnet build RustyRoguelike.sln -c Release
dotnet publish src/RustyRoguelike.NativeProduct/RustyRoguelike.NativeProduct.csproj -c Release -r linux-x64
bash src/scripts/exercise-native-product.sh
```

The small checks executable covers only the review-critical floor trust/null
boundary, full initiative cursor, defeated-actor pruning, Engine `ExactTrack`
opposition bounds, malformed restore data, and fail-atomic action and movement
settlement after navigation admission. The build checks the managed projects. Publish checks the NativeAOT shared
library. The exercise starts the actual adjacent Engine C# product runtime on
an isolated loopback port, loads the product library and committed content,
admits begin and explicit movement/Wait activations followed by demand steps, reads the
actual `rusty-roguelike.session` SSE projection to verify revision, activation,
and the current no-admitted-opposition projection, saves, perturbs, loads, and
verifies the restored observable projection and durable snapshot,
checks pause/resume/restart/shutdown lifecycle facts, reads the bundled page,
and cleans its temporary host/persistence directory.

This is an honest first-slice proof. It is not a replacement for the removed
Rust test suites, Node/Nx checks, generated protocol checks, browser E2E
workflow, or legacy CI. Add a focused C# proof only when changing an owned
behavior; do not reintroduce compatibility gates merely to preserve the old
workflow.
