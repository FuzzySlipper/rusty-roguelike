# Rusty Roguelike design

## Product boundary

Rusty Roguelike is one C# product with a collapsed party. It owns game policy:
the starter rules vocabulary, party and opposition definitions, activation and
target-selection policy, floor-artifact admission, session state, closed save
meaning, and observational game projections. It does not expose a reusable RPG
layer, depend on Rusty D20, or retain a Rust/TypeScript compatibility runtime.

Rusty Engine is the reusable mechanism provider. `RoguelikeProduct` implements
the Engine-generated `IEngineProduct` contract and receives lifecycle/update
callbacks from the Engine host. It has no second clock, raw interop, or custom
host loop. Engine owns content bytes, input delivery, deterministic random,
spatial sessions, collision/navigation, voxel-scene presentation, appearance,
UI streams, and durable state storage. The product keeps the meaning of its
rules, floor, session, and save data.

```text
committed Procgen artifact -> C# strict floor admission -> Engine content/spatial/scene
typed C# rules + party -> C# session command -> Engine random/mechanics/navigation
                           -> C# receipt and projection -> Engine UI stream / host readout
closed C# save meaning -> Engine durable state blob
```

## Current vertical slice

`RoguelikeProduct` creates an admitted starter floor and a fresh `GameSession`
at product construction. Direct Engine input intents enter a single internal,
revision-bound session boundary. The demonstrated intents are begin, cardinal
party movement, wait, save, and load. A stale, inactive, illegal, or terminal
command produces a rejected receipt and does not advance the session revision.

The party is a single C# grid cell, while each living party member retains one
place in the product-owned initiative order. The full round combines living
party members with admitted opposition in deterministic
finesse-descending/entity-ID order. Only the current party member may submit an
action; movement and wait consume that member's collapsed-party activation.
Automatic opposition settles through the next party decision, subject to the
visible settlement bound. An enemy attacks the party square and the product
chooses a living party member using that enemy's round-robin cursor. Round,
cursor, current actor, decision class, detailed receipts, and the full order
remain visible in the game projection and closed save.

Commands execute against a detached candidate session. Vitality, target
cursors, activation/round state, receipts, outcome, and revision replace the
live session only after the complete command and automatic settlement succeed.

Starter values are named in `GameplayTuning`, typed rule definitions, and
`FloorProjectionTuning`; they are intentionally inspectable and replaceable
without burying product policy in Engine mechanism calls. The initial artifact
and rules are small by design, not a claim that the campaign is feature-complete.

## Floor admission and presentation

There is no supported live C# Procgen generator. The product instead opens the
committed content artifact through `IContentService`, verifies strict schema,
hash/provenance, bounds, connectivity, required features/portals, and socket
rules, then atomically accepts it as `FloorState`. Invalid bytes do not replace
the admitted floor.

The admitted walkable cells are projected through Engine voxel, collision,
navigation, scene, and light services. `FloorProjectionTuning` carries the
product's material, grid, navigation, light, and content bounds. Engine
readouts are published separately from the game-session projection so a future
UI can inspect spatial/content/persistence facts without becoming an authority.

## Saves and presentation

`RoguelikeSaveStore` defines a closed product snapshot and codec. Engine stores
the opaque durable blob. Load validates the saved rules fingerprint and floor
provenance/content hash before replacing the live session. The host has one
development persistence root; durable profile selection is later product work.

The native product is NativeAOT and is hosted by Engine's C# product runtime.
Its small bundled HTML page is merely a lifecycle/readout endpoint. It neither
mounts a renderer nor evaluates rules. A broader first-person UX remains
deliberately unclaimed until there is a concrete product composition to build.

## Provider boundary

The sole Engine dependency is the unconditional adjacent C# project reference
to `../rusty-engine/csharp/Rusty.Engine/Rusty.Engine.csproj`, plus its public
product generator analyzer in the NativeAOT project. Do not edit or synchronize
the provider from here.

Rusty Procgen is not linked at runtime. Its reviewed public revision identifies
the provenance of the committed starter-floor artifact. A runtime generation
need is an upstream C# capability request, not a reason to copy Procgen or add
a Rust sidecar.
