import { describe, expect, it } from 'vitest';

import type { HttpClientPort } from '@rusty-roguelike/platform';
import {
  RUSTY_ENGINE_REVISION,
  RUSTY_PROCGEN_REVISION,
} from '@rusty-roguelike/protocol';

import { BootstrapTransport } from './index';

describe('BootstrapTransport', () => {
  it('strictly decodes the Rust response', async () => {
    const http: HttpClientPort = {
      get: async () => ({
        ok: true,
        status: 200,
        json: async () => ({
          schemaVersion: 1,
          product: 'rusty-roguelike',
          phase: 'bootstrap',
          rustyEngineRevision: RUSTY_ENGINE_REVISION,
          rustyProcgenRevision: RUSTY_PROCGEN_REVISION,
          procgenLinkHash: `fnv1a64:${'b'.repeat(16)}`,
        }),
      }),
    };
    await expect(new BootstrapTransport(http).load()).resolves.toMatchObject({
      product: 'rusty-roguelike',
    });
  });
});
