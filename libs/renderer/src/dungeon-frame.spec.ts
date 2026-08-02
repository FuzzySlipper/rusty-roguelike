import { describe, expect, it } from 'vitest';

import type { SessionView, TurnReceipt } from '@rusty-roguelike/protocol';

import { cameraMotionCue, createDungeonFrame } from './dungeon-frame';

const SESSION: SessionView = {
  schemaVersion: 5,
  revision: 4,
  phase: 'expedition',
  round: 2,
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
    expectedRevision: 4,
    legalSteps: ['forward'],
    canTurn: true,
    actions: [],
  },
  latestReceipts: [],
  log: [],
  world: {
    schemaVersion: 3,
    revision: 9,
    floorId: 'floor.renderer',
    facing: 'north',
    discoveredCellCount: 4,
    cells: [
      { lateral: 0, depth: 0, kind: 'floor' },
      { lateral: 0, depth: 1, kind: 'floor' },
      { lateral: 1, depth: 1, kind: 'wall' },
    ],
    scenePlacements: [],
    visibleActors: [
      {
        actorId: 'enemy.scout',
        entityId: 201,
        name: 'Scout',
        lateral: 0,
        depth: 1,
        participating: true,
      },
    ],
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
        {
          x: 4,
          y: 3,
          terrain: 'floor',
          feature: null,
          visible: true,
        },
        {
          x: 5,
          y: 3,
          terrain: 'wall',
          feature: null,
          visible: true,
        },
      ],
      visibleActors: [
        {
          actorId: 'enemy.scout',
          entityId: 201,
          name: 'Scout',
          x: 4,
          y: 3,
          participating: true,
        },
      ],
    },
  },
};

describe('createDungeonFrame', () => {
  it('maps only Rust-projected facts into stable Engine handles and metadata', () => {
    const first = createDungeonFrame(SESSION);
    const repeated = createDungeonFrame(SESSION);
    expect(repeated.handles).toEqual(first.handles);
    expect(first.frame.ops).toHaveLength(first.handles.length);
    const enemy = first.frame.ops.find(
      (operation) =>
        operation.op === 'create' &&
        operation.node.metadata.label === 'enemy-201',
    );
    expect(enemy).toMatchObject({
      op: 'create',
      node: {
        metadata: {
          sourceEntity: 201,
          tags: ['rusty-roguelike', 'enemy', 'enemy-201'],
        },
      },
    });
    expect(
      first.frame.ops.some(
        (operation) =>
          operation.op === 'create' &&
          operation.node.metadata.label === 'wall-1-1',
      ),
    ).toBe(true);
  });

  it('destroys the prior retained set before recreating the exact projection', () => {
    const first = createDungeonFrame(SESSION);
    const next = createDungeonFrame(
      {
        ...SESSION,
        revision: 5,
        world: { ...SESSION.world, visibleActors: [] },
      },
      first.handles,
    );
    expect(next.frame.ops.slice(0, first.handles.length)).toEqual(
      first.handles.map((handle) => ({ op: 'destroy', handle })),
    );
    expect(
      next.frame.ops.some(
        (operation) =>
          operation.op === 'create' &&
          operation.node.metadata.tags.includes('enemy'),
      ),
    ).toBe(false);
  });

  it('adapts only Rust-projected scene props and lights into Engine operations', () => {
    const sceneSession: SessionView = {
      ...SESSION,
      world: {
        ...SESSION.world,
        scenePlacements: [
          {
            id: 'torch.visible.prop',
            lateral: 0,
            depth: 1,
            facing: 'right',
            content: { kind: 'prop', contentId: 'prop.torch.medieval' },
          },
          {
            id: 'torch.visible.light',
            lateral: 0,
            depth: 1,
            facing: 'right',
            content: {
              kind: 'point_light',
              colorRgb: '#ffb45f',
              intensityMilli: 2500,
              rangeCells: 6,
            },
          },
        ],
      },
    };
    const frame = createDungeonFrame(sceneSession);
    expect(
      frame.frame.ops.filter((operation) => operation.op === 'createLight'),
    ).toHaveLength(2);
    expect(frame.frame.ops).toContainEqual(
      expect.objectContaining({
        op: 'defineAnimatedMesh',
        asset: expect.objectContaining({ asset: 'asset.prop.torch.medieval' }),
      }),
    );
    expect(frame.frame.ops).toContainEqual(
      expect.objectContaining({
        op: 'createAnimatedMeshInstance',
        instance: expect.objectContaining({
          asset: 'asset.prop.torch.medieval',
          metadata: expect.objectContaining({ label: 'torch.visible.prop' }),
        }),
      }),
    );
    expect(frame.handles).toHaveLength(
      frame.frame.ops.filter(
        (operation) =>
          operation.op === 'create' ||
          operation.op === 'createLight' ||
          operation.op === 'createAnimatedMeshInstance',
      ).length,
    );
  });

  it('telegraphs only the Rust-projected targets of the transient selected action', () => {
    const decision = SESSION.decision;
    if (decision === null) {
      throw new Error('fixture must expose a party decision');
    }
    const session: SessionView = {
      ...SESSION,
      decision: {
        ...decision,
        actions: [
          {
            actionId: 'aimed-shot',
            name: 'Aimed Shot',
            legalTargetEntityIds: [201],
          },
        ],
      },
    };
    const frame = createDungeonFrame(session, [], 'aimed-shot');
    const target = frame.frame.ops.find(
      (operation) =>
        operation.op === 'create' &&
        operation.node.metadata.label === 'enemy-201',
    );
    expect(target).toMatchObject({
      op: 'create',
      node: { metadata: { tags: expect.arrayContaining(['legal-target']) } },
    });
    expect(
      createDungeonFrame(session).frame.ops.some(
        (operation) =>
          operation.op === 'create' &&
          operation.node.metadata.tags.includes('legal-target'),
      ),
    ).toBe(false);
  });
});

describe('cameraMotionCue', () => {
  it('derives disposable presentation offsets only from accepted Rust receipts', () => {
    const moved: TurnReceipt = {
      kind: 'partyMoved',
      actorEntityId: 101,
      step: 'left',
    };
    const turned: TurnReceipt = {
      kind: 'partyTurned',
      actorEntityId: 101,
      direction: 'right',
    };
    expect(cameraMotionCue([moved])).toEqual({
      kind: 'step',
      lateral: -0.72,
      depth: 0,
      yawDegrees: 0,
    });
    expect(cameraMotionCue([turned])).toEqual({
      kind: 'turn',
      lateral: 0,
      depth: 0,
      yawDegrees: -90,
    });
    expect(cameraMotionCue([])).toBeNull();
  });
});
