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

## Upstream ownership

`dependency-sources.json` is the canonical source selection. Rusty Engine is
pinned for the retained renderer process and later named mechanisms. Rusty
Procgen is pinned for `rusty_procgen_preflight::core::ProcgenCore`; the consumer
never shells out to its CLI or copies its algorithms. A missing reusable
generation capability must be demonstrated by this consumer and fixed
upstream.

The bootstrap host intentionally exposes only an immutable dependency readout
and a blank retained scene. Gameplay enters in the following reviewed tasks;
the phase boundary is recorded in [known limitations](known-limitations.md).
