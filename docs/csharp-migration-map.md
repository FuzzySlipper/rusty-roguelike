# C# migration map

**Status:** target ownership map for Den task 7555.  It is a migration map,
not a promise that the C# product or every provider capability already exists.
The current Rust, build-time TypeScript, Angular, and Nx trees are donor
evidence.  The target is one ordinary NativeAOT C# product, not a port of the
Rust crate layout or a new reusable RPG framework.

## Authority and provider pins

| Owner | Keeps authority | Does not become |
| --- | --- | --- |
| `RustyRoguelike.Product` (C#) | Rules, content interpretation, admitted floor meaning, party/session/combat state, saves, receipts, tuning, and presentation facts. | An Engine wrapper, a Procgen implementation, or a generic RPG kit. |
| `Rusty.Engine` | NativeAOT lifecycle/update/input delivery, renderer/resources, spatial/navigation/collision/perception, random, persistence primitives, UI publishing, and managed mechanics/entity/resolution helpers. | The roguelike's rules or session. |
| Rusty Procgen | Deterministic generation, validation, generation diagnostics, and generated-artifact provenance. | A game-session or presentation owner. |
| TypeScript | DOM UI, accessibility, and an explicit Engine host/backend only when one is required. | A renderer, protocol authority, gameplay evaluator, or browser-side save. |

The implementation reads the generated safe `Rusty.Engine` C# contracts at
build time; ordinary product code has no P/Invoke, unsafe ABI declarations, or
raw Engine handles.  This map was made against the adjacent Engine checkout at
`94ba020e37ee1409c34f4683f96c09a144042e14` and the current Procgen checkout at
`0e937f70689abb98340642fd7f029414a3f2a6c0`.  The product remains pinned to its
reviewed Procgen public revision
`722e2c479bdf88ab39b66d2d33ab466b698ec7df` until a deliberate provider-pin
task changes it. The C# product records that exact producing revision in each
admitted floor/save; it does **not** make an Engine
checkout revision a gameplay fact.  The C# project references the one public
`Rusty.Engine` SDK surface selected by its product build; generated `obj/`
contracts are evidence, never source to copy or edit.

## Thin composition, ordinary domains

```text
src/
  RustyRoguelike.Product/
    RoguelikeProduct.cs             IEngineProduct lifecycle handoff only
    Composition/StarterSessionFactory.cs
    Rules/                          definitions, compiler, typed tuning
    Party/                          members, builds, preparation, loadout
    Floors/                         artifact admission and floor facts
    Exploration/                    collapsed pose, discovery, encounters
    Session/                        revisioned commands, turns, receipts
    Combat/                         intent, legality, damage policy
    Saves/                          closed product save/restore validation
    Presentation/                   C# projections and Engine publication
    Content/                        committed rules/floor/asset inputs
  RustyRoguelike.NativeProduct/
    NativeProduct.cs                EngineProduct attribute only
  ui/                               optional DOM/accessibility shell only
```

`RoguelikeProduct` constructs a `StarterSessionFactory` from
`ProductCreateContext`, accepts Engine-driven `Update` facts, and forwards
typed semantic input to the current session.  It does not own a clock, a
scheduler loop, rendering, or a service locator.  Each mutable domain has one
owner and a snapshot/restore shape.  A cross-domain command follows
**Read -> Decide -> Apply -> Publish** only where that boundary is useful;
direct domain methods remain preferable to a command bus.

## Source and behavior disposition

| Current donor family | C# target owner | Provider/input | Disposition and preserved behavior |
| --- | --- | --- | --- |
| `bootstrap.rs`, `lib.rs`, `bin/host.rs`, `bin/native_host.rs` | `RoguelikeProduct`, `StarterSessionFactory`, thin NativeAOT composition | Generated `IEngineProduct`, lifecycle/update/input services | **Delete/replace.** One Engine-admitted lifecycle owns start, pause, restart, shutdown, and disposal. No static Rust host, HTTP command server, Wry adapter, or second game loop survives. |
| `rules/*`, `gameplay/src/authoring/*`, `gameplay/src/catalogs/*`, `gameplay/src/packages/*`, materialized package JSON | `Rules/StarterRuleset`, `Rules/Definitions`, `Rules/RulesetCompiler`, `Content/Rules` | Engine managed `Mechanics` values/services and `Entities` only for reusable stat, track, item, inventory, and equipment mechanisms | **Adapt.** C# owns the catalog vocabulary and semantic checks. Keep definitions and provenance inspectable; TypeScript authoring and the Rust compiler are donor-only. A committed C#-readable rules artifact is content, not executable browser policy. |
| `session/loadout.rs`, party/item/class/feat catalogs | `Party/PartyState`, `Party/LoadoutState`, `Party/PreparationService` | Engine `Mechanics` inventory/equipment, stats/tracks/items; `Entities` snapshots | **Adapt.** Create unique authored items, perform preparation loadout atomically, require an empty stash plus equipped party before Begin, then make loadout read-only. Product owns body/weapon/focus slot meaning and readiness policy; Engine owns assignment and capacity truth. |
| `world/{component,state,navigation,projection}.rs` | `Exploration/ExplorationState`, `Exploration/PartyPose`, `Exploration/DiscoveryState`, `Exploration/EncounterAdmission` | Engine `Spatial`, `Perception`, `Kinematic`/`Motion`, `Entities`, `Appearance` | **Adapt.** One party pose occupies one grid cell. Dormant enemies become participating only when the product's named radius rule accepts a visible Engine `Perception.QueryVisibility` pair. The starter floor has no supplied dynamic occluders, so it deliberately asks Engine with an empty occluder set rather than fabricating walls. Movement is fail-closed until Engine task 7614 supplies side-effect-free C# navigation-step admission. Ask Engine for collision/navigation/perception facts rather than copy shadowcasting, raycasts, occupancy queries, or pathfinding. |
| `session/{types,runtime,roll}.rs` | `Session/GameSession`, `Session/TurnState`, `Session/CommandProcessor`, `Session/RollLedger` | Engine lifecycle update facts and `Random` keyed draws | **Adapt.** Every accepted action or explicit Wait consumes exactly one activation; movement remains unavailable pending Engine task 7614. Initiative, bounded automatic opposition settlement, terminal state, static/seeded roll policy, durable action index, and reject receipts remain product policy. Commands carry the observed `SessionRevision`; a detached candidate replaces the live product state only after all product work and read-only Engine calls succeed. |
| `session/resolution/*` | `Combat/AttackResolver`, `Combat/AttackPolicy`, `Combat/ResolutionReceipt`, `Combat/DamageTransaction` | Engine `Mechanics` tracks/effects and optional managed `Resolution` helper | **Adapt.** C# decides hostile-cell versus hostile-party-square legality, ability/defense facts, one-action cost, range, and damage meaning. Party and opposition vitality both use Engine `Mechanics.ExactTrack`; C# retains the damage policy and closed save meaning. An enemy attacks the party square first, then C# selects one living party member using its durable per-enemy round-robin cursor. Publish the chosen member, policy, eligible count, rolls, modifiers, defense, requested/applied damage, and rejection reason. Damage and cursor/roll/log changes are one fail-atomic product transition. |
| `floor/{authoring,generation,admission,types}.rs`, `rust/content/procgen/*` | `Floors/FloorArtifactAdmission`, `Floors/FloorState`, `Content/Floors` | Exact public Procgen artifact/input contract; Engine `Spatial`, `Voxel`, `VoxelContent`, `Appearance`, `VoxelScenePresentation` after admission | **Adapt, never copy.** Procgen consumes explicit seed, intent, geometry policy, catalog, and catalog-aware policy and produces a validated artifact. C# validates game-specific bounds, entry/goal/key/gate/portal semantics, reachable cells, known scene sockets, and full provenance before atomically replacing its floor. Props/lights remain inert placement facts; Engine owns their spatial/render realization. |
| `session/persistence.rs` and save DTOs | `Saves/RoguelikeSave`, `Saves/SaveCodec`, `Saves/RestoreValidator` | Engine `Persistence` primitives plus `Entities` snapshot/persistence helpers | **Adapt.** The closed product save includes schema, compiled-rules fingerprint, complete floor/procgen provenance, entity snapshot, phase/revision/round/outcome, initiative/cursor, roll index, target cursors, latest receipts, and full rules log. Restore constructs a fresh candidate, recompiles/re-admits rules and floor, validates identity/lifecycle/positions/inventory/receipts/log, then replaces live state once. Unknown or mismatched data never partially publishes. |
| `presentation.rs` and native retained-frame mapping | `Presentation/RoguelikeProjection`, `Presentation/WorldProjection`, `Presentation/CombatProjection` | Engine `Ui`, `Presentation`, `Appearance`, `CameraView`, `VoxelScenePresentation`, audio/animation where used | **Replace.** C# maps admitted product facts to named Engine presentation/UI calls and disposes every returned lease at its owning scope. It does not recreate retained-frame commands, webviews, mesh import, renderer resources, frame loops, or picking. Stable presentation identities, authored torch meaning, camera offsets, and projection choices remain product meaning. |
| `libs/protocol`, `libs/platform`, `libs/transport`, `libs/store` | Typed C# requests/views/receipts; optional thin `ui/` decode/render layer | C# in-process product calls and Engine-supported UI/host contract | **Delete/replace.** Rust-generated DTOs, HTTP transport, client store, and browser-side revision logic do not migrate. A DOM shell may submit explicit semantic requests and render C#-published facts, but does not calculate legality, visibility, targeting, initiative, saves, or a parallel log. |
| `libs/feature-game`, `libs/theme`, `apps/app`, `apps/app-e2e` | `ui/` only if an accessible shell is still wanted | DOM/browser host only | **Delete then recreate selectively.** Retain only accessible controls and presentation of published values. Do not preserve Angular/Nx seams, drag state as authority, native-viewport marker, renderer implementation, or old browser harness. |
| root Cargo/Nx/package/CI scripts, Rust tests, Vitest/Playwright suites | new C# solution/build/run scripts | NativeAOT product build and one explicit host route | **Delete.** They prove the retired architecture. Fresh focused checks belong beside the new product; no compatibility lane, Rust fallback, or legacy CI maintenance remains. |

## Content, tuning, and Procgen contract

Content identities and numbers stay named and close to the domain that explains
them.  Do not replace the existing observability with magic expressions or one
global constants class.

| Product-owned typed home | Named facts to retain or expose |
| --- | --- |
| `Rules/Definitions` and `StarterRuleset` | ability bounds, defense bases/contributors, damage dice/sides/bonus/kind, action target/range/tags, feat modifiers, class-level action/feat grants and slot increases, item slot/modifier/action grants, actor vitality/build/loadout. |
| `Rules/RulesetTuning` | starter package/version/fingerprint, seed or static-roll policy, one-activation cost, ID/schema bounds, and any rules-wide validation limits. |
| `Exploration/ExplorationTuning` | party movement step, visibility/perception request bounds, encounter-admission rule, dormant roster distribution (near/middle/far), blocked-cell policy, and automatic-settlement bound. |
| `Combat/CombatTuning` | party-square member-selection policy name, deterministic party order, attack evidence limits, and receipt detail level.  Per-action numbers remain in action definitions. |
| `Floors/FloorAdmissionProfile` | allowed topology/features/portal constraints, reachability requirement, known prefab socket/content IDs, light/prop interpretation, and artifact-size/attempt bounds. |

Rules and floor artifacts need explicit version, identity, source/provenance, and
canonical fingerprint/hash fields.  The product reads them as data and reports
the resolved identities in bootstrap/save diagnostics.  Procgen inputs and
generated outputs stay modular: a future floor profile may add catalog shapes,
policy, fixtures, or scene sockets without changing `GameSession`; a game
extension adds an admission/profile implementation rather than branching the
generator.  C# must not port Procgen graph construction, placement, routing,
or validation.

The current safe C# SDK does not itself expose a `ProcgenCore` generation
service.  The migration therefore needs one explicit decision before runtime
seed generation is implemented:

1. ship a committed, validated Procgen-produced floor artifact (and consume it
   through `FloorArtifactAdmission`); or
2. request a narrow upstream, purpose-neutral, safe generation-artifact
   capability and stop that slice until it exists.

Do not hide that gap behind a C# Procgen port, a Rust sidecar, shell-out, or
browser simulation.  The committed-artifact route preserves the seed, inputs,
selected attempt, hashes, and exact Procgen revision needed for save restore.

## Cutover and deletion gates

1. Add the two C# projects, thin composition, domain definitions, and an
   artifact-backed starter floor.  Establish the C# content/save contracts
   before any donor runtime is removed.
2. Land the first vertical session: preparation -> begin -> one revision-bound
   action or move -> receipt/projection -> save/reopen.  It is the only
   required behavioral proof shape; do not recreate the old exhaustive suite.
3. Wire the Engine host lifecycle and any minimal DOM accessibility shell.  A
   real NativeAOT launch must show that C# receives Engine updates and releases
   scopes/leases at shutdown/restart.
4. Delete the Rust gameplay crate, build-time gameplay TypeScript, generated
   protocol, Angular/Nx applications, old test projects, package lockfiles,
   workflows, and obsolete CI scripts in one cutover after the C# route is the
   only supported route.  Do not leave compatibility adapters or dual hosts.
5. Replace only the useful gates: C# build/publish, a short real host scenario,
   artifact/provenance admission, and a search that finds no active legacy
   runtime imports.  Provider-wide legacy checks and broad coverage are not
   migration acceptance criteria.

Before each deletion, confirm the C# replacement owns the behavior.  The
minimal acceptance matrix is: lifecycle/disposal uses Engine ownership;
generated C# contracts are regenerated rather than edited; stale/failed
commands leave session and Engine state unchanged; saves reject mismatched
content/floor provenance; a Procgen artifact fails before floor publication;
and the browser cannot become a gameplay or renderer authority.

## Known migration gaps

- **Runtime Procgen generation:** no current safe C# generation service is
  identified.  Artifact admission is a valid first slice; runtime regeneration
  needs the upstream capability described above.
- **Exact current Engine call shapes:** capability names in this map route work
  only.  Each implementation slice must read the generated `Rusty.Engine`
  contract at the pinned checkout and use supported calls, not inferred Rust
  internals.
- **Native and DOM presentation composition:** choose a concrete product UX
  before combining them.  A DOM shell is optional and remains observational;
  no downstream renderer bridge is a permissible interim solution.

## Donor consultation

- **Corpus and snapshots:** current Rusty Roguelike donor at
  `909dd4521e57ddebd277522b7e887534a34fa23d`; adjacent Engine C# guidance/SDK
  inspected at `94ba020e37ee1409c34f4683f96c09a144042e14`; product-reviewed Procgen
  contract pinned at `722e2c479bdf88ab39b66d2d33ab466b698ec7df`, with the newer local
  Procgen checkout `0e937f70689abb98340642fd7f029414a3f2a6c0` consulted only to confirm
  that no supported C# generation surface exists.
- **Files/flows inspected:** `docs/design.md`, `docs/agent-code-atlas.md`,
  rules/session/world/floor source families, gameplay catalog/authoring
  grammar, Engine `AGENTS.md`, `docs/csharp-sdk.md`,
  `docs/csharp-capabilities.md`, and the implemented Rusty Dagger migration
  map as a boundary example.
- **Outcome:** adapted.  Preserve the collapsed-party and receipt/save/floor
  semantics, but translate them into one product-specific C# domain layout;
  reject the donor Rust/TypeScript runtime topology and any generic RPG layer.
