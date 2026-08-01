import { fileURLToPath } from 'node:url';

import { defineConfig } from 'vitest/config';

function workspacePath(path: string): string {
  return fileURLToPath(new URL(path, import.meta.url));
}

export default defineConfig({
  resolve: {
    alias: {
      '@rusty-roguelike/platform': workspacePath(
        './libs/platform/src/index.ts',
      ),
      '@rusty-roguelike/protocol': workspacePath(
        './libs/protocol/src/index.ts',
      ),
      '@rusty-roguelike/transport': workspacePath(
        './libs/transport/src/index.ts',
      ),
    },
  },
  test: {
    exclude: ['apps/app-e2e/**', 'node_modules/**', 'tools/**'],
  },
});
