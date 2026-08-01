import { describe, expect, it } from 'vitest';

import type { HttpClientPort } from '@rusty-roguelike/platform';
import {
  RUSTY_ENGINE_REVISION,
  RUSTY_PROCGEN_REVISION,
} from '@rusty-roguelike/protocol';

import {
  BootstrapTransport,
  SessionTransport,
  SessionTransportError,
} from './index';

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
      post: async () => {
        throw new Error('unexpected post');
      },
    };
    await expect(new BootstrapTransport(http).load()).resolves.toMatchObject({
      product: 'rusty-roguelike',
    });
  });
});

const TERMINAL_SESSION = {
  schemaVersion: 1,
  revision: 3,
  round: 2,
  outcome: 'victory',
  current: null,
  order: [],
  party: [
    {
      entityId: 101,
      actorId: 'party.brann',
      name: 'Brann',
      currentVitality: 20,
      maximumVitality: 20,
      conscious: true,
      carriedItems: [{ itemId: 'item.sword', name: 'Sword' }],
    },
  ],
  decision: null,
  latestReceipts: [],
  world: {
    schemaVersion: 1,
    revision: 12,
    floorId: 'floor.transport',
    facing: 'north',
    discoveredCellCount: 1,
    cells: [{ lateral: 0, depth: 0, kind: 'floor' }],
    visibleActors: [],
  },
};

describe('SessionTransport', () => {
  it('posts typed commands and strictly decodes the session', async () => {
    let observed: unknown;
    const http: HttpClientPort = {
      get: async () => ({
        ok: true,
        status: 200,
        json: async () => TERMINAL_SESSION,
      }),
      post: async (_url, body) => {
        observed = body;
        return {
          ok: true,
          status: 200,
          json: async () => TERMINAL_SESSION,
        };
      },
    };
    const transport = new SessionTransport(http);
    await expect(transport.load()).resolves.toEqual(TERMINAL_SESSION);
    await expect(
      transport.command({
        kind: 'turnRight',
        actorEntityId: 101,
        expectedRevision: 2,
      }),
    ).resolves.toEqual(TERMINAL_SESSION);
    expect(observed).toEqual({
      kind: 'turnRight',
      actorEntityId: 101,
      expectedRevision: 2,
    });
  });

  it('preserves classified Rust command rejection', async () => {
    const http: HttpClientPort = {
      get: async () => {
        throw new Error('unexpected get');
      },
      post: async () => ({
        ok: false,
        status: 409,
        json: async () => ({
          code: 'session_revision_stale',
          detail: 'the command revision is stale',
        }),
      }),
    };
    await expect(
      new SessionTransport(http).command({
        kind: 'turnLeft',
        actorEntityId: 101,
        expectedRevision: 1,
      }),
    ).rejects.toEqual(
      new SessionTransportError(
        'session_revision_stale',
        409,
        'the command revision is stale',
      ),
    );
  });
});
