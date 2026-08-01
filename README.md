# Rusty Roguelike

Rusty Roguelike is an independent first-person grid roguelike reference game.
Its party occupies one square behind the camera, visible participants resolve
single-action activations in initiative order, and combat stays in the
exploration renderer rather than switching to a tactical screen.

The current foundation combines an intentionally blank, real Rust-served
renderer shell with deterministic Rust-owned admission of a compact floor from
the public Rusty Procgen core. Later milestones connect that admitted floor to
the collapsed-party session and playable browser projection.

```bash
pnpm install --frozen-lockfile
./scripts/verify.sh
pnpm run serve:local
```

See [the design](docs/design.md), [verification](docs/verification.md), and
[known limitations](docs/known-limitations.md).
