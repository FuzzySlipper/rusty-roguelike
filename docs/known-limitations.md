# Known limitations

## Rust-only renderer migration

- **Status:** resolved
- **Affected surface:** task 6700, native product host, browser shell
- **Limitation:** the former Angular renderer imported four Engine TypeScript
  packages and directly mounted the renderer.
- **Impact:** resolved; downstream TypeScript no longer knows the renderer
  implementation language or private bridge.
- **Detection:** the dependency gate rejects every `@rusty-engine/*` package,
  while `pnpm run verify:native` exercises the fixed Rust adapter.
- **Follow-up:** none.
- **Introduced by:** task 6700 intermediate facade/projection milestone
- **Last reviewed:** 2026-08-08 / codex

## Native and browser presentation are separate product surfaces

- **Status:** active
- **Affected surface:** `rusty-roguelike-native`, Angular browser shell
- **Limitation:** the native product window owns the first-person renderer and
  semantic keyboard/pick controls, while the browser build remains the richer
  party, minimap, preparation, and rules-log presentation. They are two entry
  points and do not share one live process session.
- **Impact:** the native host is the renderer-authoritative product proof, but
  it does not yet reproduce every informational Angular overlay.
- **Reason left in place:** task 6700 isolates the sensitive renderer boundary
  without inventing a downstream webview composition framework.
- **Detection:** `pnpm run native` opens the native renderer; `pnpm run
serve:local` opens the observational browser shell.
- **Follow-up:** add a product-specific combined shell only when concrete UX
  evidence justifies it; do not expose or copy the Engine bridge.
- **Introduced by:** task 6700
- **Last reviewed:** 2026-08-08 / codex

The repository now proves its independent boundary, exact public dependencies,
strict bootstrap protocol, real Rust host, retained renderer lifecycle, and
deterministic Rust-owned admission of one bounded Procgen floor. It also owns a
strict Ruleweaver-inspired starter catalog, compiled party/enemies, registered
Engine components, a named-mechanics-service proof, and Rust-authoritative
collapsed-party movement, visibility, discovery, dormancy, restore validation,
and a split local-scene/minimap protocol that never leaks occluded actors.
It also owns a live Rust-hosted initiative session with exactly one movement,
action, turn, or explicit Wait per activation, authoritative seeded/static rolls,
direct Engine stat/damage
resolution, automatic opposition movement, and bounded no-legal progression.
Opposition attacks now target the collapsed party square, select a living member
with Rust-owned per-enemy round-robin fairness, and publish a complete strictly
decoded resolution receipt. The current catalog does not author area or
multi-member effects.

- The browser control interface includes authoritative preparation, shared-stash
  drag/drop plus click-select equipment assignment, complete tabbed party
  inspection, the live collapsed-party expedition, action alternatives, party
  vitality,
  read-only field packs, and detailed rules receipts.
- The native renderer retains one bounded offscreen local-overview target over
  the same Rust-admitted retained scene for lookup. It is deliberately not
  composited into the primary view or used as the
  detailed minimap: it has no discovered-map memory, visibility computation,
  icons, input, CPU readback, post-processing, or gameplay authority. The
  accessible polished minimap is the sole visible presentation of Rust's
  separate minimap DTO.
- Expedition loadouts are intentionally read-only. A later equipment-in-turn
  design must assign an explicit activation cost before mutation can be admitted;
  this phase does not treat opening Packs as a free equipment action.
- New sessions create and equip the canonical party items before first
  publication, leaving the preparation stash empty. The bounded floor does not
  yet author dropped loot or consumables; preparation only supports optional
  unequip, transfer, and re-equip customization of that starting loadout.

The admitted floor and compiled rules form the live Rust `WorldState` and
`GameSession`; the host publishes their strict projection and accepts typed,
revision-bound gameplay commands. The browser's single Game menu routes Save,
Load, and New / Restart through those Rust lifecycle operations. Exit is
intentionally disabled in the web build because a browser cannot truthfully
close its own tab; closing the native product window disposes its Engine child
surface before exit. Complete schema-4 saves include the registered
Engine entity snapshot, exact floor/provenance and content identities, initiative,
RNG and target cursors, inventory/equipment, progression facts, terminal state,
and the complete bounded Rust log. The current same-origin host save slot is
intentionally process-local in both product entry points; durable
filesystem/profile selection remains future product policy.

Prefab scene sockets currently place a single optimized medieval-torch prop and
paired warm point light in selected room shapes. The world-neutral Engine rig is
disabled, so these Rust-projected lights are the only dungeon illumination and
their native retained-frame admission is covered by the real product-host
proof. The GLB is converted by Engine Rust import code into a content-addressed
static mesh while preserving the authored placement transform; the proof also
requires corrupt packed bytes to prevent renderer readiness. Lighting remains
a fixed authored first pass: there is no generated
light-density analysis, adaptive
fill-light placement, emissive flame animation, or prop collision.
The torch donor is CC BY-NC-SA 4.0, so the bundled derivative is non-commercial
and share-alike; its attribution and license ship beside the GLB.

The real-host product proof now completes the bounded floor: it starts from the
canonical equipped loadout, optionally unequips and restores Scale Mail through
the empty shared stash, begins the continuous initiative expedition,
observes each hidden raider join only after discovery and the next round rebuild,
uses an equipment-granted action, records Rust-selected party-member damage,
saves and reopens active combat, reaches Rust-owned victory, and reopens the
terminal save. The objective panel is a presentation of `SessionOutcome`; it
does not own a parallel completion rule.
