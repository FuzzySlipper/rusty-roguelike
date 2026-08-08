# Rusty Roguelike

Rusty Roguelike is an independent first-person grid roguelike reference game.
Its party occupies one square behind the camera, visible participants resolve
single-action activations in initiative order, and combat stays in the
exploration renderer rather than switching to a tactical screen.

The native product renders Rust-owned retained frames through Rusty Engine's
fixed Rust Wry adapter. The Angular application is an observational gameplay
and accessibility shell; it contains no Engine renderer packages or bootstrap.

```bash
pnpm install --frozen-lockfile
./scripts/verify.sh
pnpm run native
pnpm run serve:local
```

See [the design](docs/design.md), [verification](docs/verification.md), and
[known limitations](docs/known-limitations.md).
