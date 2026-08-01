import { describe, expect, it } from 'vitest';

import type { SessionCommandDto, SessionView } from '@rusty-roguelike/protocol';
import { SessionTransportError } from '@rusty-roguelike/transport';

import { SessionStoreCore, type SessionTransportPort } from './index';

const SESSION: SessionView = {
  schemaVersion: 1,
  revision: 0,
  round: 1,
  outcome: 'ongoing',
  current: {
    entityId: 101,
    actorId: 'party.brann',
    name: 'Brann',
    side: 'party',
    initiative: 12,
  },
  order: [
    {
      entityId: 101,
      actorId: 'party.brann',
      name: 'Brann',
      side: 'party',
      initiative: 12,
    },
  ],
  party: [
    {
      entityId: 101,
      actorId: 'party.brann',
      name: 'Brann',
      currentVitality: 20,
      maximumVitality: 20,
      conscious: true,
      carriedItems: [],
    },
  ],
  decision: {
    actorEntityId: 101,
    expectedRevision: 0,
    legalSteps: ['forward'],
    canTurn: true,
    actions: [],
  },
  latestReceipts: [],
  world: {
    schemaVersion: 1,
    revision: 1,
    floorId: 'floor.store',
    facing: 'north',
    discoveredCellCount: 1,
    cells: [{ lateral: 0, depth: 0, kind: 'floor' }],
    visibleActors: [],
  },
};

describe('SessionStoreCore', () => {
  it('admits only one delayed mutation and permits a later command after settlement', async () => {
    let release: ((value: SessionView) => void) | undefined;
    const commands: SessionCommandDto[] = [];
    const transport: SessionTransportPort = {
      load: async () => SESSION,
      command: (command) => {
        commands.push(command);
        return new Promise<SessionView>((resolve) => {
          release = resolve;
        });
      },
    };
    const store = new SessionStoreCore(transport);
    await store.load();
    const command: SessionCommandDto = {
      kind: 'turnRight',
      actorEntityId: 101,
      expectedRevision: 0,
    };

    const first = store.command(command);
    expect(store.busy()).toBe(true);
    await expect(store.command(command)).resolves.toBe(false);
    expect(commands).toEqual([command]);
    expect(store.commandError()).toBeNull();

    const decision = SESSION.decision;
    if (decision === null) {
      throw new Error('fixture must expose a party decision');
    }
    release?.({
      ...SESSION,
      revision: 1,
      decision: { ...decision, expectedRevision: 1 },
      latestReceipts: [
        { kind: 'partyTurned', actorEntityId: 101, direction: 'right' },
      ],
    });
    await expect(first).resolves.toBe(true);
    expect(store.busy()).toBe(false);
    expect(store.log()).toHaveLength(1);

    const second = store.command({ ...command, expectedRevision: 1 });
    expect(commands).toHaveLength(2);
    release?.({ ...SESSION, revision: 2 });
    await expect(second).resolves.toBe(true);
  });

  it('keeps the admitted view and exposes a classified command failure', async () => {
    const transport: SessionTransportPort = {
      load: async () => SESSION,
      command: async () => {
        throw new SessionTransportError(
          'session_revision_stale',
          409,
          'the command revision is stale',
        );
      },
    };
    const store = new SessionStoreCore(transport);
    await store.load();

    await expect(
      store.command({
        kind: 'turnLeft',
        actorEntityId: 101,
        expectedRevision: 0,
      }),
    ).resolves.toBe(false);
    expect(store.state()).toEqual({ status: 'ready', value: SESSION });
    expect(store.commandError()).toEqual({
      code: 'session_revision_stale',
      detail: 'the command revision is stale',
    });
    expect(store.busy()).toBe(false);
  });
});
