# Verification

Run the full local gate from the repository root:

```bash
pnpm install --frozen-lockfile
./scripts/verify.sh
```

The gate proves exact dependency carriers, Rust formatting/tests/clippy, strict
protocol generation, browser boundaries, lint/typecheck/unit tests, production
build, and real Rust-served Chromium at desktop and mobile sizes.

Use an isolated port when another product host owns the default:

```bash
E2E_PORT=4429 pnpm run verify:browser
```

The browser report under `dist/.playwright/` contains the inspected screenshots.
