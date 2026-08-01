import { defineConfig, devices } from '@playwright/test';
import { workspaceRoot } from '@nx/devkit';
import { nxE2EPreset } from '@nx/playwright/preset';

const port = process.env['E2E_PORT'] ?? '4417';
const baseURL = `http://127.0.0.1:${port}`;

export default defineConfig({
  ...nxE2EPreset(import.meta.dirname, { testDir: './src' }),
  fullyParallel: false,
  workers: 1,
  use: { baseURL, trace: 'on-first-retry' },
  webServer: {
    command: `cargo run --manifest-path rust/Cargo.toml -p rusty-roguelike --bin rusty-roguelike-host -- --address 127.0.0.1:${port}`,
    url: `${baseURL}/healthz`,
    reuseExistingServer: false,
    cwd: workspaceRoot,
    timeout: 240_000,
  },
  projects: [
    { name: 'desktop-chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'mobile-chromium', use: { ...devices['Pixel 7'] } },
  ],
});
