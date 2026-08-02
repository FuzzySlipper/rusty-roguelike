# Rusty Roguelike design

## Product boundary

Rusty Roguelike is one independent game. It is not a Rusty D20 mode, a shared
RPG framework, or a facade over either Rusty Engine or Rusty Procgen.

The durable direction is:

```text
authored game policy + explicit generation seed
  -> public Rusty Procgen in-memory generation
  -> Rust admission of generated floor and content
  -> registered state plus named Rusty Engine services
  -> Rust collapsed-party session and initiative runtime
  -> Rust-generated strict protocol
  -> browser store, renderer, features, and presentation
```

Rust owns generated-floor admission, discovery and visibility, participation
and dormancy, the collapsed party square, initiative, the one-action economy,
movement, actions, target/member selection, inventory, progression, objectives,
saves, and projection. TypeScript may translate typed observations into a
frame and keep transient focus, animation, and input state. It does not decide
gameplay legality or maintain a second live session.

## Preparation and party inventory

A session begins in an explicit preparation phase before initiative is live.
Rust creates every authored item as a unique registered Engine entity, then a
fresh `GameSession` atomically transfers and equips the authored party loadout
through the named inventory and equipment services. The first published view
therefore has an empty shared expedition stash and is ready to begin without
browser-issued setup commands. Party members and the stash expose Engine
inventory capacity; party members additionally expose the Roguelike-authored
body, weapon, and focus slots through Engine equipment state. Revision-bound
loadout commands still use those named services on a cloned world, so players
may customize before beginning and a stale owner, incompatible slot, full
destination, or service failure cannot partially publish state.

The expedition becomes ready only when the shared stash is empty and every
party item is equipped. Beginning the expedition is a separate authoritative
command that freezes the loadout, builds initiative, and settles automatic
actors to the first party decision. Equipment-granted actions and modifiers
become live only from the equipped registered item facts. Once the expedition
begins, the inventory and complete character sheets remain inspectable but
read-only: changing gear is not a free side channel around the one-activation
economy.

## Turn model

One movement step or one chosen action consumes one activation. Rounds retain
initiative order. Each currently participating visible actor receives an
activation; hidden or non-participating enemies remain dormant. When a party
member activates, movement relocates the whole collapsed party. An enemy first
targets the party square and Rust then selects which party member receives the
effect.

Combat never changes to a modal tactical board. The first-person renderer,
party status, action controls, and log remain one composition throughout
exploration and initiative resolution.

The bounded starter-floor objective is to find and defeat all fifteen authored
dormant raiders, distributed deterministically across near, middle, and far
reachable floor strata so contact begins early without collapsing the roster at
the entry. Living enemies occupy distinct cells, treat the collapsed party cell
and every other living actor as blocked, and stop on an available adjacent cell
before attacking. An actor with no legal route consumes its activation without
moving. Rust derives victory only from authoritative actor vitality across the
exact compiled opposition roster; the browser's objective panel merely presents
the current `SessionOutcome`. Terminal sessions accept no further gameplay
commands, but their complete state can still be saved and reopened through the
host lifecycle.

## Rules compilation

`rust/content/rules/starter.json` is inert authored policy. Rust strictly decodes
it into the Roguelike-owned candidate schema, requires exact Engine rules-package
provenance for every definition, resolves the admitted package, validates all
cross-references and bounds, and compiles an immutable `RoguelikeRuleset`.
TypeScript declarations are generated from that Rust schema; the browser does
not compile or evaluate the candidate.

The rules retain the Ruleweaver-inspired attack-versus-defense shape, four
ability/defense families, class level grants, feats, equipment, and attributed
modifiers. They intentionally omit D20's multi-budget activation model. Movement
and every attack compile with the same fixed activation cost of one. The later
session runtime owns when that activation is spent.

## Initiative session

`GameSession` owns one continuous, non-modal initiative order. Finesse orders
live party members and permanently participating enemies, with entity identity
as the stable tie break. A successful relative step, rotation, or selected
attack consumes the current party member's sole activation; party movement
relocates the collapsed party square. Failed and stale commands operate on a
cloned candidate session and publish neither world state, turn cursor, nor roll
consumption.

Opposition activations settle automatically through Engine-routed movement to
the next party decision. An actor with no currently legal action explicitly
passes, defeated actors are removed, newly revealed actors join on the next
round rebuild, and automatic settlement has a fixed bound. Seeded action rolls
come from the pinned Engine RNG service under one stable scope per action index;
authored static rolls are consumed in order and must match the selected action's
dice. The durable action index is cross-validated against the complete Rust
rules log. The current catalog authors no
reactions, so there is no acknowledgement pause. Enemy attacks intentionally
target the collapsed party square rather than an aggregate party resource. For
each enemy, Rust selects one living party member in authored party order using
a per-enemy round-robin cursor, then evaluates that member's defense and applies
damage to that member through the named Engine services. The staged cursor,
rolls, damage, and full resolution receipt publish atomically. Receipts expose
the selected member, selection policy, eligible count, rolls, modifiers,
defense, and requested versus applied damage; the browser displays those facts
and never chooses the recipient. The starter catalog has no area or multi-member
effect, so no area targeting policy is implied.

Durable actor abilities, builds, and collapsed-party membership use registered
`entity-state` components. Defense modifiers, vitality bounds, damage kinds,
items, and equipment use an admitted `gameplay-mechanics` catalog and named
Engine services directly. This repository owns their Roguelike vocabulary and
does not wrap those mechanisms in a shared RPG facade.

## Complete session persistence

Save schema 3 is a closed Rust-owned contract. It records the exact public
Engine and Procgen revisions, compiled starter-rules fingerprint, admitted floor
and complete Procgen provenance, Engine's registered durable entity snapshot,
session revision/round/phase/outcome, derived initiative order and cursor,
action-roll index, per-enemy target cursors, latest receipts, and the complete
bounded rules log. Session view schema 5 projects that Rust log directly; the
browser no longer assembles a parallel history from transient receipts.

Restore recompiles the starter rules and regenerates the floor from the saved
seed before comparing either artifact. It restores registered entities through
the exact world component registry, validates compiled entity and immutable
component identity, Engine mechanics/catalog consistency, reachable discovery
and actor positions, living occupancy, dormancy, vitality, inventory/equipment,
initiative, terminal outcome, receipt arithmetic, damage history, RNG position,
and log continuity. Unknown nested facts, old schemas, dependency mismatches,
forged provenance, disconnected positions, impossible components, and
inconsistent lifecycle facts reject before a replacement session is published.
Finally, Rust replays every initiating receipt from a fresh authored session and
requires exact receipts, registered snapshot, initiative, rolls, cursors, log,
and projection; locally plausible but unreachable combinations therefore fail.
The same-origin host owns one in-memory save slot; Save captures the current
Rust session and Reopen constructs a fresh `GameSession` from its serialized
contract before atomically replacing live state.

## Collapsed-party world state

The admitted floor is projected into Engine voxel, navigation, and collision
services. Rust stores one party pose and facing for the whole collapsed party;
one accepted relative step relocates that pose atomically. Party discovery and
each enemy's floor position and dormant/participating state are durable
registered components. Every admitted walkable cell and every restored durable
position must be reachable from the authored entry through the same Engine
navigation projection used by movement.

Visibility is a bounded forward cone produced by deterministic recursive
shadowcasting and constrained by Engine collision raycasts. Walls, corners, and
authored locked portal fixtures occlude later cells while the first blocker
remains observable.
Seeing a dormant enemy promotes it permanently into encounter participation;
turning away removes it from the current visible projection without putting it
back to sleep. Party discovery durably records observed floor and wall facts
separately. The browser-facing WorldView contains the relative visible
first-person facts plus a bounded Rust-projected minimap of only discovered
terrain, known feature/door icons, the party pose, and currently visible
opposition. It never contains undiscovered topology or hidden enemy positions.
The generated TypeScript decoder rejects unknown, out-of-cone, contradictory,
or minimap-only current facts rather than becoming another visibility
authority.

## Upstream ownership

`dependency-sources.json` is the canonical source selection. Rusty Engine is
pinned for the retained renderer process, registered entity state, rules-package
admission, and named gameplay mechanics. Rusty
Procgen is pinned for `rusty_procgen_preflight::core::ProcgenCore`; the consumer
never shells out to its CLI or copies its algorithms. A missing reusable
generation capability must be demonstrated by this consumer and fixed
upstream.

## Generated-floor admission

The authored seed intent, layout policy, catalog, and bounded catalog-aware
policy live under `rust/content/procgen`. Rust parses them into Procgen's public
types and runs the complete filesystem-free `ProcgenCore` pipeline in memory.
The game admits only a successful unit-cell, four-way, bounded, connected
lock/key floor with the expected entry, goal, key, gate, and portal semantics.
Floor schema 2 additionally admits Procgen-authored prefab scene sockets as
inert prop and point-light placements. Rust requires the exact validated
catalog placement chain, bounded known content, unique placement identity,
walkable placement cells, and paired torch/light sockets before publication.
The scene placements have no collision, navigation, visibility, or gameplay
authority.

Every admitted floor retains the exact Procgen revision, authored seed and
derived stage seeds, selected attempt, plus canonical hashes for all authored,
intermediate, accepted, and result artifacts. Admission recomputes those hashes
before trusting geometry. A proposed generation is fully generated and
admitted before it can replace the current floor, so malformed, incompatible,
or exhausted results cannot partially publish state.

The Rust host publishes the live session and accepts only strict,
revision-bound typed commands plus explicit save/reopen lifecycle requests. Its
projection includes phase, preparation
readiness, the shared stash while preparing, complete party identity, class,
level, experience, abilities, defenses, feats, actions and loadouts, the current
decision, relative visible topology, and complete target-resolution receipts.
The browser strictly decodes that view, keeps classified transport failures
visible, and submits only projected choices; it does not recreate inventory,
equipment, movement, targeting, initiative, or rules policy.

One permanent public Engine `RendererSurface` owns the full window. World view
schema 3 projects only scene placements whose authored cells are currently
visible floor facts; hidden prefab topology and lights never cross the protocol.
A pure adapter maps those scene placements, relative cells, and visible actors
into stable retained handles, public animated-mesh/light operations, and public
picking metadata. A low ambient term provides ordinary 3D readability while
authored prefab point lights provide local warm illumination. Brief camera offsets are derived
from accepted movement/turn receipts and discarded after presentation; reduced
motion snaps immediately. The preparation workbench, initiative, movement,
action selection, tabbed party sheet, field packs, and detailed rules log are
overlays around that same canvas. Preparation offers native drag/drop plus a
click-select destination alternative and busy-gates every mutation surface. The
expedition Party/Packs disclosures are truthful nonmodal, keyboard-operable,
read-only regions. Selecting an action highlights only its Rust-projected legal
targets; resolved attacks drive bounded retained impact presentation. Enemy
selection is an Engine metadata pick with explicit button and keyboard
alternatives, and all resulting legality remains admitted by Rust.
