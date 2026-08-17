# Rusty Roguelike gameplay authoring

This workspace is the build-time home for Rusty Roguelike gameplay
definitions: abilities, defenses, damage types, actions, feats, classes,
items, actors, and the collapsed party. TypeScript here is an authoring
language only — it never evaluates. Rust
(`rust/crates/rusty-roguelike/src/rules`) admits the materialized package,
owns its meaning, and is the only evaluator. See
`/home/dev/rusty-engine/docs/topics/gameplay/downstream-adoption.md`.

## Layout

- `src/authoring/` — the grammar: definition shapes, builders, and envelope
  composition. `mod.ts` is the single import surface for catalogs.
- `src/catalogs/` — the everyday editing surface, one file per section
  (`abilities`, `defenses`, `damageTypes`, `actions`, `feats`, `classes`,
  `items`, `actors`, `party`). Entries read as data with builder helpers, not
  control flow.
- `src/packages/` — one entry per package composing catalogs into the
  deterministic envelope. Materialization walks this directory.
- `scripts/materialize.mjs` — deterministic build plumbing emitting
  `data/gameplay/<domain>-<package>.package.json`.

House rules:

- Catalogs import only from `../authoring/mod.js`.
- Adding content (an ability, action, feat, item, actor, …) is a one-file
  catalog edit. Extending the grammar itself means editing `authoring/` and
  the Rust compiler in `rust/crates/rusty-roguelike/src/rules/` in the same
  change — that coupling is intentional.
- Fixed gameplay policy is not authored: movement actions move exactly one
  grid step and target self only; the `ally-cell` target is rejected by the
  compiler and absent from the grammar; activation cost is compiler-hardcoded
  to 1.
- Content matches `rust/content/rules/starter.json` exactly; the materialized
  payload is semantically identical to that file (the envelope adds wrapper
  fields only).

## Commands

```bash
pnpm gameplay:build   # typecheck, compile, materialize data/gameplay/*.package.json
pnpm gameplay:check   # build + verify the committed package has no drift
```
