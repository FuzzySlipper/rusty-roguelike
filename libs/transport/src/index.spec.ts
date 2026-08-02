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
  schemaVersion: 5,
  revision: 3,
  phase: 'expedition',
  round: 2,
  outcome: 'victory',
  current: null,
  order: [],
  party: [
    {
      entityId: 101,
      actorId: 'party.brann',
      name: 'Brann',
      title: 'Shield of the Lantern',
      level: 1,
      experience: 0,
      classId: 'guardian',
      className: 'Guardian',
      classLevel: 1,
      currentVitality: 20,
      maximumVitality: 20,
      conscious: true,
      abilities: [{ abilityId: 'might', score: 16, modifier: 3 }],
      defenses: [{ defenseId: 'armor', value: 16 }],
      feats: [
        {
          featId: 'shield-discipline',
          name: 'Shield Discipline',
          description: 'Hold the line.',
        },
      ],
      actions: [{ actionId: 'shield-bash', name: 'Shield Bash' }],
      loadout: {
        ownerEntityId: 101,
        inventorySlots: [null, null, null],
        equipmentSlots: [
          { slotId: 'body', label: 'Body', equipped: null },
          { slotId: 'weapon', label: 'Weapon', equipped: null },
          { slotId: 'focus', label: 'Focus', equipped: null },
        ],
        capacity: { used: 0, maximum: 3 },
      },
    },
  ],
  preparation: null,
  decision: null,
  latestReceipts: [],
  log: [],
  world: {
    schemaVersion: 4,
    revision: 12,
    floorId: 'floor.transport',
    facing: 'north',
    discoveredCellCount: 1,
    cells: [{ lateral: 0, depth: 0, kind: 'floor' }],
    scenePlacements: [],
    visibleActors: [],
    minimap: {
      party: { x: 4, y: 4 },
      facing: 'north',
      cells: [
        {
          x: 4,
          y: 4,
          terrain: 'floor',
          feature: null,
          visible: true,
        },
      ],
      visibleActors: [],
    },
  },
};

describe('SessionTransport', () => {
  it('posts typed commands and strictly decodes the session', async () => {
    let observed: unknown;
    const urls: string[] = [];
    const http: HttpClientPort = {
      get: async () => ({
        ok: true,
        status: 200,
        json: async () => TERMINAL_SESSION,
      }),
      post: async (url, body) => {
        urls.push(url);
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
    await expect(transport.save()).resolves.toEqual(TERMINAL_SESSION);
    await expect(transport.reopen()).resolves.toEqual(TERMINAL_SESSION);
    expect(urls).toEqual([
      '/api/v1/session/commands',
      '/api/v1/session/save',
      '/api/v1/session/reopen',
    ]);
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
