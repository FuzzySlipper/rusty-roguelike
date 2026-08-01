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

Durable actor abilities, builds, and collapsed-party membership use registered
`entity-state` components. Defense modifiers, vitality bounds, damage kinds,
items, and equipment use an admitted `gameplay-mechanics` catalog and named
Engine services directly. This repository owns their Roguelike vocabulary and
does not wrap those mechanisms in a shared RPG facade.

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

Every admitted floor retains the exact Procgen revision, authored seed and
derived stage seeds, selected attempt, plus canonical hashes for all authored,
intermediate, accepted, and result artifacts. Admission recomputes those hashes
before trusting geometry. A proposed generation is fully generated and
admitted before it can replace the current floor, so malformed, incompatible,
or exhausted results cannot partially publish state.

The browser still exposes only the bootstrap readout and blank retained scene.
Session visibility and playable projection enter in later reviewed tasks; the
phase boundary is recorded in [known limitations](known-limitations.md).
