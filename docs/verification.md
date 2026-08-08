# Verification

Run the full local gate from the repository root:

```bash
pnpm install --frozen-lockfile
./scripts/verify.sh
```

The gate proves exact dependency carriers, Rust formatting/tests/clippy, strict
protocol generation, browser boundaries, lint/typecheck/unit tests, production
build, a real X11/WebKit native renderer host, and Rust-served Chromium at
desktop and mobile sizes.

`pnpm run verify:native` mounts the Engine-owned private artifact through the
real Rust adapter and certifies frame submission, the bounded overview and
first-person camera, resize, one content-hash-bound product resource, physical
input readback, picking, state readback, explicit rendering, an authoritative
command plus exact save/restore round trip, and acknowledged disposal. On
Linux the proof uses Xvfb with WebKit compositing disabled for deterministic
software-rendered teardown.

The desktop browser path starts from the complete canonical equipped loadout,
then proves optional preparation by unequipping and restoring Scale Mail before
using the equipment-granted Focus Orb action. It enters the generated floor,
observes dormant enemies join only after discovery and a round rebuild,
encounters all fifteen
opponents across the generated floor, resolves the complete hostile roster
without a modal combat screen, consumes an explicit Wait through Space/click
while preserving disclosure keyboard behavior, inspects Rust-selected
party-member damage,
uses the game menu to save/load and restart, saves/reopens active combat,
reaches victory, and reopens the terminal save. It
also proves that exactly one polished detailed minimap follows Rust discovery,
remembered versus current visibility, enemy appearance, revision replacement,
and classified failure nonmutation. The browser also proves it neither fetches
the torch GLB nor bootstraps an Engine renderer; those responsibilities are
covered by the native proof. The mobile path repeats
preparation and lifecycle controls, keyboard input, a visible torch/light pair,
responsive panel separation, the upper-right map/menu cluster, 44-pixel
controls, and viewport containment.

Use an isolated port when another product host owns the default:

```bash
E2E_PORT=4429 pnpm run verify:browser
```

For a campaign certification that bypasses Nx result reuse:

```bash
NX_SKIP_NX_CACHE=true E2E_PORT=4429 ./scripts/verify.sh
```

The browser report under `dist/.playwright/` contains the named preparation and
expedition screenshots for both viewport projects. Preparation begins with the
canonical party loadout equipped and ready; the scenario unequips and restores
Scale Mail through the click/drag alternatives before starting immediately.
