# Agent code atlas

| Path                                                            | Owner                                                                                  | Focused proof                                         |
| --------------------------------------------------------------- | -------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| `dependency-sources.json`, `tools/check-dependency-sources.mjs` | Exact public Engine and Procgen identities and carrier audit                           | `pnpm run check:dependencies`                         |
| `rust/content/procgen/`                                         | Authored seed intent, compact shape catalog, and bounded generation policy             | Rust floor tests                                      |
| `rust/crates/rusty-roguelike/src/bootstrap.rs`                  | Immutable dependency readout and generated bootstrap protocol owner                    | Rust bootstrap tests                                  |
| `rust/crates/rusty-roguelike/src/floor/authoring.rs`            | Strict embedded authoring input decoding                                               | Rust floor tests                                      |
| `rust/crates/rusty-roguelike/src/floor/`                        | Direct public `ProcgenCore` pipeline, provenance, prefab scene-socket admission        | Rust floor tests                                      |
| `rust/crates/rusty-roguelike/src/floor/admission.rs`            | Roguelike-owned bounded topology, semantic, and provenance admission                   | Rust floor tests                                      |
| `rust/crates/rusty-roguelike/src/floor/types.rs`                | Admitted floor DTO, exact provenance, and atomic floor replacement                     | Rust floor tests                                      |
| `rust/content/rules/starter.json`                               | Inert Roguelike-owned starter rules, party, enemies, classes, feats, items             | Rust rules tests                                      |
| `rust/crates/rusty-roguelike/src/rules/candidate.rs`            | Strict candidate schema, package envelope, generated TypeScript owner                  | Rust rules tests; protocol check                      |
| `rust/crates/rusty-roguelike/src/rules/compiler.rs`             | Semantic compilation, provenance, references, one-activation rule                      | Rust rules tests                                      |
| `rust/crates/rusty-roguelike/src/rules/mechanics.rs`            | Direct Engine mechanics catalog projection                                             | Engine `StatService` integration test                 |
| `rust/crates/rusty-roguelike/src/rules/component.rs`            | Durable registered actor/build/collapsed-party components                              | Rust rules tests                                      |
| `rust/crates/rusty-roguelike/src/world/component.rs`            | Durable party pose/observed terrain and enemy placement/participation components       | Rust world restore tests                              |
| `rust/crates/rusty-roguelike/src/world/navigation.rs`           | Engine navigation/collision, bounded local scene topology, and forward shadowcasting   | Rust world movement/visibility tests                  |
| `rust/crates/rusty-roguelike/src/world/state.rs`                | Atomic collapsed-party movement, discovery, dormancy, and restore authority            | Rust world lifecycle tests                            |
| `rust/crates/rusty-roguelike/src/world/projection.rs`           | Bounded relative world and discovery-safe detailed minimap DTOs                        | Rust world projection tests; protocol decoder tests   |
| `rust/crates/rusty-roguelike/src/session/types.rs`              | Session commands, activation/order views, receipts, durable log, failures              | Rust session tests                                    |
| `rust/crates/rusty-roguelike/src/session/roll.rs`               | Seeded/static authoritative action-roll source                                         | Rust roll atomicity tests                             |
| `rust/crates/rusty-roguelike/src/session/runtime.rs`            | Initiative order, cursor, bounded automatic settlement, and terminal state             | Rust session lifecycle tests                          |
| `rust/crates/rusty-roguelike/src/session/persistence.rs`        | Closed complete save, fresh-process restore, identity and lifecycle validation         | Rust save/reopen and forgery tests                    |
| `rust/crates/rusty-roguelike/src/session/loadout.rs`            | Engine inventory/equipment projection, preparation moves, and ready admission          | Rust loadout atomicity tests                          |
| `rust/crates/rusty-roguelike/src/session/resolution.rs`         | Party commands, Engine attacks, member selection, and opposition AI                    | Rust movement/action/target-fairness tests            |
| `rust/crates/rusty-roguelike/src/lib.rs`                        | Private module facade and public Rust API                                              | `cargo test --manifest-path rust/Cargo.toml --locked` |
| `rust/crates/rusty-roguelike/src/bin/host.rs`                   | Same-origin query/command/save/reopen/restart and static Rust host                     | real Playwright lifecycle                             |
| `libs/protocol/`                                                | Generated bootstrap/rules/world/session DTOs and strict JSON admission                 | Vitest; `pnpm run protocol:check`                     |
| `libs/platform/`                                                | Browser HTTP, frames, input, resize, motion, drag-data, and pixel-ratio ports          | typecheck; consumers                                  |
| `libs/transport/`                                               | Strict bootstrap/session transport and classified command failures                     | Vitest                                                |
| `libs/store/`                                                   | Bounded async command/save/reopen admission and Rust-log publication                   | delayed-transport Vitest; browser lifecycle           |
| `libs/renderer/`, `apps/app/public/assets/torch/`               | Rust-view retained adapter, Engine mesh/light ops, picking, tween, asset lifecycle     | Vitest; real WebGL expedition and asset-failure proof |
| `libs/feature-game/src/preparation.ts`                          | Busy-safe preparation composition and revision-bound loadout commands                  | real desktop/mobile preparation                       |
| `libs/feature-game/src/loadout-panel.ts`                        | Presentational drag/drop and click-select loadout surface                              | real desktop/mobile preparation                       |
| `libs/feature-game/src/party-sheet.ts`                          | Read-only keyboard-tabbed complete character and loadout inspection                    | real desktop/mobile expedition                        |
| `libs/feature-game/src/minimap.ts`                              | Presentational detailed map of the strict Rust discovery/visibility projection         | minimap unit proof; real desktop/mobile expedition    |
| `libs/feature-game/src/index.ts`                                | Full-window canvas plus expedition action/status/log/objective composition             | real desktop/mobile expedition and terminal victory   |
| `libs/theme/`                                                   | Product tokens and global geometry                                                     | build; browser smoke                                  |
| `apps/app/`                                                     | Angular application binding only                                                       | build                                                 |
| `apps/app-e2e/`                                                 | Real Rust-served preparation, full-floor victory, save/reopen, and responsive UI proof | `pnpm run verify:browser`                             |

Do not use this atlas instead of the owning design or executable contract.
