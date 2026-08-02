import { describe, expect, it } from 'vitest';

import type { SessionCommandDto, SessionView } from '@rusty-roguelike/protocol';
import { SessionTransportError } from '@rusty-roguelike/transport';

import { SessionStoreCore, type SessionTransportPort } from './index';

const SESSION: SessionView = {
  schemaVersion: 5,
  revision: 0,
  phase: 'expedition',
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
  decision: {
    actorEntityId: 101,
    expectedRevision: 0,
    legalSteps: ['forward'],
    canTurn: true,
    actions: [],
  },
  latestReceipts: [],
  log: [],
  world: {
    schemaVersion: 4,
    revision: 1,
    floorId: 'floor.store',
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

describe('SessionStoreCore', () => {
  it('publishes only Rust-returned save and reopen state', async () => {
    const saved: SessionView = {
      ...SESSION,
      revision: 1,
      decision:
        SESSION.decision === null
          ? null
          : { ...SESSION.decision, expectedRevision: 1 },
      latestReceipts: [
        {
          kind: 'partyTurned' as const,
          actorEntityId: 101,
          direction: 'right' as const,
        },
      ],
      log: [
        {
          id: 1,
          revision: 1,
          receipt: {
            kind: 'partyTurned' as const,
            actorEntityId: 101,
            direction: 'right' as const,
          },
        },
      ],
    };
    let live = saved;
    const transport: SessionTransportPort = {
      load: async () => live,
      command: async () => live,
      save: async () => live,
      reopen: async () => saved,
    };
    const store = new SessionStoreCore(transport);
    await store.load();
    await expect(store.save()).resolves.toBe(true);
    expect(store.persistenceNotice()).toBe('Session saved.');
    live = { ...saved, revision: 2, latestReceipts: [], log: saved.log };
    await expect(store.reopen()).resolves.toBe(true);
    expect(store.state()).toEqual({ status: 'ready', value: saved });
    expect(store.log()).toEqual(saved.log);
    expect(store.persistenceNotice()).toBe('Saved session reopened.');
  });

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
      save: async () => SESSION,
      reopen: async () => SESSION,
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
      log: [
        {
          id: 1,
          revision: 1,
          receipt: {
            kind: 'partyTurned',
            actorEntityId: 101,
            direction: 'right',
          },
        },
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
      save: async () => SESSION,
      reopen: async () => SESSION,
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
