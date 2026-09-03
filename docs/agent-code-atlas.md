# Agent code atlas

| Path | Owner | Focused proof |
| --- | --- | --- |
| `src/RustyRoguelike.Product/RustyRoguelike.Product.csproj` | ordinary SDK product composition and bundle facts | `rusty dev` CoreCLR stage |
| `src/RustyRoguelike.Product.Checks/` | narrow review-critical floor/session executable checks | focused checks command in `docs/verification.md` |
| `src/RustyRoguelike.Product/RoguelikeProduct.cs` | Engine lifecycle composition, input routing, projection publication | host exercise |
| `src/RustyRoguelike.Product/Rules/` | Roguelike vocabulary and named gameplay tuning | build + session readout |
| `src/RustyRoguelike.Product/Party/`, `Combat/`, `Exploration/`, `Session/` | collapsed-party state, one-activation policy, dormancy, receipts, restore shape | host exercise |
| `src/RustyRoguelike.Product/Floors/` | strict artifact admission and provenance policy | host startup admits committed content |
| `src/RustyRoguelike.Product/Integration/FloorEngineProjection.cs` | product composition over Engine content, spatial, voxel scene, navigation, and lights | host exercise |
| `src/RustyRoguelike.Product/Saves/` | closed product save contract over Engine persistence | begin/save/load host exercise |
| `src/RustyRoguelike.Product/Presentation/` | observational game, lifecycle, and Engine readout streams | host lifecycle/readout |
| `ui/`, `content/` | product-owned DOM readout and admitted content | SDK staging + packaged host |
| `src/scripts/exercise-product.sh` | maintained packaged CoreCLR real-host proof | `bash src/scripts/exercise-product.sh` |
| `src/RustyRoguelike.NavigationAtomicityProbe/` | explicit NativeAOT navigation fidelity probe | `bash src/scripts/exercise-navigation-atomicity.sh` |
| `dependency-sources.json` | reviewed Procgen artifact provenance | manual source review when artifact changes |
| `docs/csharp-migration-map.md` | donor-to-C# ownership and removal dispositions | cutover review |

There are no active Rust gameplay, generated protocol, TypeScript gameplay,
Angular/Nx, Node, browser-E2E, or legacy CI lanes. Do not recreate them as
wrappers around the C# product. New product code belongs under the C# project
that owns its policy; reusable mechanisms belong upstream in Rusty Engine.
