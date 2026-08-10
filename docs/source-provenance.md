# Source provenance

## Runtime dependencies

- Rusty Engine: one unconditional adjacent path dependency on
  `../../rusty-engine/rust/crates/rusty-engine`. The product consumes the local
  checkout exactly as it stands, declares only the complete `rusty-engine` Rust
  facade, and reaches every Engine owner through its preserved
  `rusty_engine::<owner>` namespace. Roguelike scripts never pull, synchronize,
  or mutate that checkout. Engine services own the floor's voxel, navigation,
  movement, visibility, deterministic random, and renderer-host mechanisms;
  the game retains Roguelike policy, action resolution, state, frame
  projection, shell, and presentation meaning.
- Historical renderer-boundary migration provenance: the previous browser renderer packages came
  from Engine revision `04970a44ef2e87a3453f086469deff64f5ae56f4`.
  Task 6700 removed that complete JavaScript dependency graph and its lockfile
  records. That SHA remains historical migration provenance. The current
  replacement uses only the adjacent Rust facade; its Engine-owned Wry adapter
  embeds the private renderer artifact upstream.
- Rusty Procgen: `https://github.com/FuzzySlipper/rusty-procgen` at
  `722e2c479bdf88ab39b66d2d33ab466b698ec7df`. Rust links the public
  filesystem-free `rusty_procgen_preflight::core::ProcgenCore` facade and
  consumes its validated prefab scene-socket placements as inert generation
  facts. Torch content and rendering remain owned by this game.

`rust/Cargo.toml`, `Cargo.lock`, and the dependency boundary check prove the
adjacent Engine facade carrier. `dependency-sources.json` and `Cargo.lock`
remain the executable exact-identity proof for Procgen.

## Procgen authoring inputs

The first-floor intent and bounded policies are Rusty Roguelike-owned. The
compact catalog selectively adapts the public catalog schema and two room
shapes from Rusty Procgen's `fixtures/shape-catalogs/2d-basic.json` at the
pinned revision: `shape.room.flow_junction.spaced_8_exit` and
`shape.room.pocket.9x9_west_south`. The remaining small threshold and corridor
shapes are game-authored data. The generation code invokes only public
`ProcgenCore` operations; it does not reproduce Procgen placement or routing
algorithms.

The lock/key seed intent was informed by Rusty Procgen's
`fixtures/intents/first-slice.intent.json` at the same revision and was
rewritten for this game's one-floor contract.

## Donor evidence

The initial Nx/Angular package seams, Rust static-host pattern, retained abstract
scene, and browser-proof approach were adapted from Rusty D20 through exact
revision `2ef818e180abf507b3af7fd9bc1029f1e0983237`. Names, contracts, code, and
ownership were reduced and rewritten for this product. Rusty D20 is not a
runtime, build, or test dependency.

The preparation loadout interaction was informed by Rusty D20 revision
`cba4918f96fe6a58a8e3e3682800a39ecaeaf9ca`, and the complete party inspection
surface by revision `cfb9b69859718ecf9adf4623f5dba1a3ba887183`. Rusty Roguelike
retains no copied protocol or runtime authority: it rewrites the interactions
for unique Engine item entities, its own three-slot vocabulary, explicit
preparation admission, and a read-only expedition boundary.

The starter rules selectively adapt concepts from Ruleweaver revision
`04ef26d0eef1ba478a2c39b78cca61fe82b15be5`, especially
`docs/action-economy-conditions-targeting.md`, `docs/class-talent-system.md`,
`docs/items-inventory-party.md`, and `docs/action-system-architecture.md` plus
the longsword, shortbow, leather armor, Defensive Mobility, and Uncanny Dodge
definitions inspected at that revision. Rusty Roguelike rewrites those ideas
around one activation per movement/action and owns the resulting candidate,
catalog, party, enemy, class, feat, item, and action vocabulary. Ruleweaver is
not a runtime or build dependency.

Donor evidence never overrides this repository's design.

## Presentation asset

Historical presentation provenance: Rusty Engine revision
`d0b5e672b83d463bff71d8d35c877f770142ff3c` is the revision at which the native presentation boundary
entered this product. The adjacent Engine facade now supplies the
renderer-neutral retained model, bounded view-composition contracts, packaged
private renderer artifact, and fixed Rust Wry adapter.
The bundled torch is an optimized derivative of
[Medieval Torch - Free](https://sketchfab.com/3d-models/medieval-torch-free-065861234a824cb982764f04627331c9)
by [Typhen](https://sketchfab.com/typhen). It is licensed CC BY-NC-SA 4.0;
the attribution, modification note, license link, and non-commercial/share-alike
restriction ship beside the GLB in `apps/app/public/assets/torch`.
