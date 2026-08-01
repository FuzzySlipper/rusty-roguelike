# Agent code atlas

| Path                                                            | Owner                                                                     | Focused proof                                         |
| --------------------------------------------------------------- | ------------------------------------------------------------------------- | ----------------------------------------------------- |
| `dependency-sources.json`, `tools/check-dependency-sources.mjs` | Exact public Engine and Procgen identities and carrier audit              | `pnpm run check:dependencies`                         |
| `rust/crates/rusty-roguelike/src/lib.rs`                        | Rust bootstrap readout, Procgen library linkage, generated protocol owner | `cargo test --manifest-path rust/Cargo.toml --locked` |
| `rust/crates/rusty-roguelike/src/bin/host.rs`                   | Same-origin static/API Rust host                                          | real Playwright smoke                                 |
| `libs/protocol/`                                                | Generated DTO and strict unknown-JSON admission                           | Vitest; `pnpm run protocol:check`                     |
| `libs/platform/`                                                | Browser HTTP, resize, and device-pixel-ratio ports                        | typecheck; consumers                                  |
| `libs/transport/`                                               | Typed bootstrap transport                                                 | Vitest                                                |
| `libs/store/`                                                   | Angular async bootstrap state                                             | Vitest; browser smoke                                 |
| `libs/renderer/`                                                | Retained abstract bootstrap frame and public Engine surface lifecycle     | Vitest; real WebGL smoke                              |
| `libs/feature-game/`                                            | Full-window renderer and presentation overlay composition                 | real browser smoke                                    |
| `libs/theme/`                                                   | Product tokens and global geometry                                        | build; browser smoke                                  |
| `apps/app/`                                                     | Angular application binding only                                          | build                                                 |
| `apps/app-e2e/`                                                 | Real Rust-served desktop/mobile certification                             | `pnpm run verify:browser`                             |

Do not use this atlas instead of the owning design or executable contract.
