# Agent code atlas

| Path                                                            | Owner                                                                       | Focused proof                                         |
| --------------------------------------------------------------- | --------------------------------------------------------------------------- | ----------------------------------------------------- |
| `dependency-sources.json`, `tools/check-dependency-sources.mjs` | Exact public Engine and Procgen identities and carrier audit                | `pnpm run check:dependencies`                         |
| `rust/content/procgen/`                                         | Authored seed intent, compact shape catalog, and bounded generation policy  | Rust floor tests                                      |
| `rust/crates/rusty-roguelike/src/bootstrap.rs`                  | Immutable dependency readout and generated bootstrap protocol owner         | Rust bootstrap tests                                  |
| `rust/crates/rusty-roguelike/src/floor/authoring.rs`            | Strict embedded authoring input decoding                                    | Rust floor tests                                      |
| `rust/crates/rusty-roguelike/src/floor/generation.rs`           | Direct public `ProcgenCore` generation pipeline and provenance capture      | Rust floor tests                                      |
| `rust/crates/rusty-roguelike/src/floor/admission.rs`            | Roguelike-owned bounded topology, semantic, and provenance admission        | Rust floor tests                                      |
| `rust/crates/rusty-roguelike/src/floor/types.rs`                | Admitted floor DTO, exact provenance, and atomic floor replacement          | Rust floor tests                                      |
| `rust/content/rules/starter.json`                               | Inert Roguelike-owned starter rules, party, enemies, classes, feats, items  | Rust rules tests                                      |
| `rust/crates/rusty-roguelike/src/rules/candidate.rs`            | Strict candidate schema, package envelope, generated TypeScript owner       | Rust rules tests; protocol check                      |
| `rust/crates/rusty-roguelike/src/rules/compiler.rs`             | Semantic compilation, provenance, references, one-activation rule           | Rust rules tests                                      |
| `rust/crates/rusty-roguelike/src/rules/mechanics.rs`            | Direct Engine mechanics catalog projection                                  | Engine `StatService` integration test                 |
| `rust/crates/rusty-roguelike/src/rules/component.rs`            | Durable registered actor/build/collapsed-party components                   | Rust rules tests                                      |
| `rust/crates/rusty-roguelike/src/world/component.rs`            | Durable party pose/discovery and enemy placement/participation components   | Rust world restore tests                              |
| `rust/crates/rusty-roguelike/src/world/navigation.rs`           | Public Engine navigation and collision projections over admitted floors     | Rust world movement/occlusion tests                   |
| `rust/crates/rusty-roguelike/src/world/state.rs`                | Atomic collapsed-party movement, discovery, dormancy, and restore authority | Rust world lifecycle tests                            |
| `rust/crates/rusty-roguelike/src/world/projection.rs`           | Bounded relative occlusion-safe world DTO                                   | Rust world projection tests; protocol decoder tests   |
| `rust/crates/rusty-roguelike/src/lib.rs`                        | Private module facade and public Rust API                                   | `cargo test --manifest-path rust/Cargo.toml --locked` |
| `rust/crates/rusty-roguelike/src/bin/host.rs`                   | Same-origin static/API Rust host                                            | real Playwright smoke                                 |
| `libs/protocol/`                                                | Generated bootstrap/rules/world DTOs and strict unknown-JSON admission      | Vitest; `pnpm run protocol:check`                     |
| `libs/platform/`                                                | Browser HTTP, resize, and device-pixel-ratio ports                          | typecheck; consumers                                  |
| `libs/transport/`                                               | Typed bootstrap transport                                                   | Vitest                                                |
| `libs/store/`                                                   | Angular async bootstrap state                                               | Vitest; browser smoke                                 |
| `libs/renderer/`                                                | Retained abstract bootstrap frame and public Engine surface lifecycle       | Vitest; real WebGL smoke                              |
| `libs/feature-game/`                                            | Full-window renderer and presentation overlay composition                   | real browser smoke                                    |
| `libs/theme/`                                                   | Product tokens and global geometry                                          | build; browser smoke                                  |
| `apps/app/`                                                     | Angular application binding only                                            | build                                                 |
| `apps/app-e2e/`                                                 | Real Rust-served desktop/mobile certification                               | `pnpm run verify:browser`                             |

Do not use this atlas instead of the owning design or executable contract.
