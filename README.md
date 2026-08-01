# Rusty Roguelike

Rusty Roguelike is an independent first-person grid roguelike reference game.
Its party occupies one square behind the camera, visible participants resolve
single-action activations in initiative order, and combat stays in the
exploration renderer rather than switching to a tactical screen.

The first milestone is an intentionally blank, real Rust-served renderer shell.
It proves the public Rusty Engine and Rusty Procgen dependency boundaries that
later gameplay milestones build upon.

```bash
pnpm install --frozen-lockfile
./scripts/verify.sh
pnpm run serve:local
```

See [the design](docs/design.md), [verification](docs/verification.md), and
[known limitations](docs/known-limitations.md).
