import { describe, expect, it } from 'vitest';

import {
  decodeBootstrapReadout,
  decodeSessionView,
  decodeWorldView,
  RUSTY_ENGINE_REVISION,
  RUSTY_PROCGEN_REVISION,
} from './index';

const VALID = {
  schemaVersion: 1,
  product: 'rusty-roguelike',
  phase: 'bootstrap',
  rustyEngineRevision: RUSTY_ENGINE_REVISION,
  rustyProcgenRevision: RUSTY_PROCGEN_REVISION,
  procgenLinkHash: `fnv1a64:${'a'.repeat(16)}`,
};

describe('decodeBootstrapReadout', () => {
  it('accepts the exact Rust-owned bootstrap shape', () => {
    expect(decodeBootstrapReadout(VALID)).toEqual(VALID);
  });

  it('rejects unknown fields and dependency drift', () => {
    expect(() => decodeBootstrapReadout({ ...VALID, extra: true })).toThrow(
      'missing or unknown',
    );
    expect(() =>
      decodeBootstrapReadout({
        ...VALID,
        rustyProcgenRevision: '0'.repeat(40),
      }),
    ).toThrow('wrong Rusty Procgen');
    expect(() =>
      decodeBootstrapReadout({ ...VALID, procgenLinkHash: 'a'.repeat(64) }),
    ).toThrow('invalid Procgen linkage hash');
  });
});

const WORLD_VIEW = {
  schemaVersion: 3,
  revision: 17,
  floorId: 'floor.occlusion-regression',
  facing: 'north',
  discoveredCellCount: 8,
  cells: [
    { lateral: 0, depth: 0, kind: 'floor' },
    { lateral: 0, depth: 1, kind: 'floor' },
    { lateral: 0, depth: 2, kind: 'wall' },
  ],
  scenePlacements: [],
  visibleActors: [],
  minimap: {
    party: { x: 2, y: 4 },
    facing: 'north',
    cells: [
      {
        x: 2,
        y: 4,
        terrain: 'floor',
        feature: 'entry',
        visible: true,
      },
      { x: 2, y: 3, terrain: 'floor', feature: null, visible: true },
      { x: 2, y: 2, terrain: 'wall', feature: null, visible: true },
    ],
    visibleActors: [],
  },
};

const VISIBLE_ACTOR = {
  actorId: 'enemy.scout',
  entityId: 201,
  name: 'Scout',
  lateral: 0,
  depth: 1,
  participating: true,
};
const MINIMAP_ACTOR = {
  actorId: 'enemy.scout',
  entityId: 201,
  name: 'Scout',
  x: 2,
  y: 3,
  participating: true,
};
const WORLD_WITH_ACTOR = {
  ...WORLD_VIEW,
  visibleActors: [VISIBLE_ACTOR],
  minimap: { ...WORLD_VIEW.minimap, visibleActors: [MINIMAP_ACTOR] },
};

describe('decodeWorldView', () => {
  it('accepts the bounded relative Rust projection', () => {
    expect(decodeWorldView(WORLD_VIEW)).toEqual(WORLD_VIEW);
    expect(decodeWorldView(WORLD_WITH_ACTOR)).toEqual(WORLD_WITH_ACTOR);
    const withTorch = {
      ...WORLD_VIEW,
      scenePlacements: [
        {
          id: 'scene.torch.1.prop',
          lateral: 0,
          depth: 1,
          facing: 'right',
          content: { kind: 'prop', contentId: 'prop.torch.medieval' },
        },
        {
          id: 'scene.torch.1.light',
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
    };
    expect(decodeWorldView(withTorch)).toEqual(withTorch);
  });

  it('rejects absolute or occluded topology and malformed actor facts', () => {
    expect(() =>
      decodeWorldView({
        ...WORLD_VIEW,
        cells: [
          ...WORLD_VIEW.cells,
          { lateral: 0, depth: 3, kind: 'floor', x: 2 },
        ],
      }),
    ).toThrow('missing or unknown');
    expect(() =>
      decodeWorldView({
        ...WORLD_VIEW,
        cells: [...WORLD_VIEW.cells, { lateral: 0, depth: 3, kind: 'floor' }],
      }),
    ).toThrow('behind an occluding wall');
    expect(() =>
      decodeWorldView({
        ...WORLD_VIEW,
        cells: [...WORLD_VIEW.cells, { lateral: 0, depth: 7, kind: 'floor' }],
      }),
    ).toThrow('relative depth');
    expect(() =>
      decodeWorldView({
        ...WORLD_VIEW,
        visibleActors: [
          {
            actorId: 'enemy.scout',
            entityId: 201,
            name: 'Scout',
            lateral: 0,
            depth: 1,
            participating: false,
            position: { x: 1, y: 0 },
          },
        ],
      }),
    ).toThrow('missing or unknown');
    expect(() =>
      decodeWorldView({
        ...WORLD_VIEW,
        visibleActors: [
          {
            actorId: 'enemy.scout',
            entityId: 201,
            name: 'Scout',
            lateral: 0,
            depth: 1,
            participating: false,
          },
        ],
      }),
    ).toThrow('participation fact');
    expect(() =>
      decodeWorldView({
        ...WORLD_VIEW,
        visibleActors: [{ ...VISIBLE_ACTOR, depth: 3 }],
      }),
    ).toThrow('projected floor fact');
    expect(() =>
      decodeWorldView({
        ...WORLD_VIEW,
        visibleActors: [{ ...VISIBLE_ACTOR, depth: 2 }],
      }),
    ).toThrow('projected floor fact');
    expect(() =>
      decodeWorldView({
        ...WORLD_VIEW,
        visibleActors: [{ ...VISIBLE_ACTOR, lateral: 1 }],
      }),
    ).toThrow('projected floor fact');
    expect(() =>
      decodeWorldView({
        ...WORLD_WITH_ACTOR,
        visibleActors: [
          VISIBLE_ACTOR,
          { ...VISIBLE_ACTOR, actorId: 'enemy.second', entityId: 202 },
        ],
        minimap: {
          ...WORLD_WITH_ACTOR.minimap,
          visibleActors: [
            MINIMAP_ACTOR,
            { ...MINIMAP_ACTOR, actorId: 'enemy.second', entityId: 202 },
          ],
        },
      }),
    ).toThrow('overlapping visible actor');
    expect(() =>
      decodeWorldView({
        ...WORLD_VIEW,
        visibleActors: [{ ...VISIBLE_ACTOR, lateral: 0, depth: 0 }],
        minimap: {
          ...WORLD_VIEW.minimap,
          visibleActors: [
            {
              ...MINIMAP_ACTOR,
              x: WORLD_VIEW.minimap.party.x,
              y: WORLD_VIEW.minimap.party.y,
            },
          ],
        },
      }),
    ).toThrow('overlapping visible actor');
  });

  it('rejects leaked or malformed authored scene placements', () => {
    const prop = {
      id: 'scene.torch.1.prop',
      lateral: 0,
      depth: 1,
      facing: 'right',
      content: { kind: 'prop', contentId: 'prop.torch.medieval' },
    };
    expect(() =>
      decodeWorldView({
        ...WORLD_VIEW,
        scenePlacements: [{ ...prop, depth: 3 }],
      }),
    ).toThrow('projected floor fact');
    expect(() =>
      decodeWorldView({
        ...WORLD_VIEW,
        scenePlacements: [{ ...prop, depth: 2 }],
      }),
    ).toThrow('projected floor fact');
    expect(() =>
      decodeWorldView({
        ...WORLD_VIEW,
        scenePlacements: [
          { ...prop, content: { ...prop.content, contentId: 'prop.forged' } },
        ],
      }),
    ).toThrow('unknown content identity');
    expect(() =>
      decodeWorldView({
        ...WORLD_VIEW,
        scenePlacements: [
          {
            ...prop,
            content: {
              kind: 'point_light',
              colorRgb: '#ffb45f',
              intensityMilli: 2500,
              rangeCells: 13,
            },
          },
        ],
      }),
    ).toThrow('scene light range');
    expect(() =>
      decodeWorldView({
        ...WORLD_VIEW,
        scenePlacements: [{ ...prop, browserOwned: true }],
      }),
    ).toThrow('missing or unknown');
  });

  it('rejects hidden or contradictory minimap facts at the browser boundary', () => {
    expect(() =>
      decodeWorldView({
        ...WORLD_VIEW,
        minimap: {
          ...WORLD_VIEW.minimap,
          visibleActors: [{ ...MINIMAP_ACTOR, x: 1 }],
        },
      }),
    ).toThrow('do not match');
    expect(() =>
      decodeWorldView({
        ...WORLD_VIEW,
        minimap: {
          ...WORLD_VIEW.minimap,
          cells: [
            ...WORLD_VIEW.minimap.cells,
            { x: 2, y: 1, terrain: 'floor', feature: null, visible: true },
          ],
        },
      }),
    ).toThrow('current terrain');
    expect(() =>
      decodeWorldView({
        ...WORLD_VIEW,
        minimap: {
          ...WORLD_VIEW.minimap,
          cells: [
            ...WORLD_VIEW.minimap.cells,
            { x: 1, y: 4, terrain: 'wall', feature: 'goal', visible: false },
          ],
        },
      }),
    ).toThrow('wall cannot carry');
  });
});

const ACTIVATION = {
  entityId: 102,
  actorId: 'party.kestrel',
  name: 'Kestrel',
  side: 'party',
  initiative: 16,
};

const OPPOSITION_ATTACK = {
  kind: 'oppositionAttacked',
  actorEntityId: 202,
  actionId: 'rusty-blade',
  target: {
    selectedMemberEntityId: 102,
    selectionPolicy: 'round-robin-living',
    eligibleMemberCount: 3,
  },
  d20: 15,
  abilityModifier: 2,
  attackTotal: 17,
  defense: 14,
  hit: true,
  damageRolls: [4],
  damageBonus: 1,
  requestedDamage: 5,
  appliedDamage: 5,
};

const SESSION_VIEW = {
  schemaVersion: 5,
  revision: 4,
  phase: 'expedition',
  round: 2,
  outcome: 'ongoing',
  current: ACTIVATION,
  order: [ACTIVATION],
  party: [
    {
      entityId: 102,
      actorId: 'party.kestrel',
      name: 'Kestrel',
      title: 'Pathfinder',
      level: 1,
      experience: 0,
      classId: 'scout',
      className: 'Scout',
      classLevel: 1,
      currentVitality: 18,
      maximumVitality: 18,
      conscious: true,
      abilities: [{ abilityId: 'finesse', score: 17, modifier: 3 }],
      defenses: [{ defenseId: 'armor', value: 14 }],
      feats: [
        {
          featId: 'defensive-mobility',
          name: 'Defensive Mobility',
          description: 'Stay difficult to pin down.',
        },
      ],
      actions: [{ actionId: 'aimed-shot', name: 'Aimed Shot' }],
      loadout: {
        ownerEntityId: 102,
        inventorySlots: [
          {
            entityId: 302,
            itemId: 'longbow',
            name: 'Longbow',
            equipmentSlotId: 'weapon',
            equippedSlotId: 'weapon',
          },
          null,
        ],
        equipmentSlots: [
          { slotId: 'body', label: 'Body', equipped: null },
          {
            slotId: 'weapon',
            label: 'Weapon',
            equipped: {
              entityId: 302,
              itemId: 'longbow',
              name: 'Longbow',
              equipmentSlotId: 'weapon',
              equippedSlotId: 'weapon',
            },
          },
          { slotId: 'focus', label: 'Focus', equipped: null },
        ],
        capacity: { used: 1, maximum: 2 },
      },
    },
  ],
  preparation: null,
  decision: {
    actorEntityId: 102,
    expectedRevision: 4,
    legalSteps: ['forward'],
    canTurn: true,
    actions: [
      {
        actionId: 'aimed-shot',
        name: 'Aimed Shot',
        legalTargetEntityIds: [201],
      },
    ],
  },
  latestReceipts: [OPPOSITION_ATTACK],
  log: [{ id: 1, revision: 4, receipt: OPPOSITION_ATTACK }],
  world: WORLD_WITH_ACTOR,
};

const STASH_ITEM = {
  entityId: 303,
  itemId: 'leather-armor',
  name: 'Leather Armor',
  equipmentSlotId: 'body',
  equippedSlotId: null,
};

const PREPARATION_VIEW = {
  ...SESSION_VIEW,
  revision: 0,
  phase: 'preparation',
  round: 1,
  current: null,
  order: [],
  decision: null,
  latestReceipts: [],
  log: [],
  preparation: {
    ready: false,
    stash: {
      ownerEntityId: 204,
      inventorySlots: [STASH_ITEM, null],
      equipmentSlots: [],
      capacity: { used: 1, maximum: 2 },
    },
  },
};

describe('decodeSessionView', () => {
  it('accepts the complete Rust-selected party-square attack receipt', () => {
    expect(decodeSessionView(SESSION_VIEW)).toEqual(SESSION_VIEW);
    expect(decodeSessionView(PREPARATION_VIEW)).toEqual(PREPARATION_VIEW);
  });

  it('strictly admits preparation, loadout ownership, capacity, and equipment facts', () => {
    const member = SESSION_VIEW.party[0];
    const feat = member?.feats[0];
    if (member === undefined || feat === undefined) {
      throw new Error('session fixture must include a party member and feat');
    }
    expect(() =>
      decodeSessionView({
        ...PREPARATION_VIEW,
        preparation: {
          ...PREPARATION_VIEW.preparation,
          ready: true,
        },
      }),
    ).toThrow('ready fact');
    expect(() =>
      decodeSessionView({
        ...SESSION_VIEW,
        party: [
          {
            ...member,
            loadout: {
              ...member.loadout,
              capacity: { used: 2, maximum: 2 },
            },
          },
        ],
      }),
    ).toThrow('usage disagrees');
    expect(() =>
      decodeSessionView({
        ...SESSION_VIEW,
        party: [
          {
            ...member,
            loadout: {
              ...member.loadout,
              equipmentSlots: member.loadout.equipmentSlots.map((slot) =>
                slot.slotId === 'weapon'
                  ? {
                      ...slot,
                      equipped: { ...slot.equipped, entityId: 999 },
                    }
                  : slot,
              ),
            },
          },
        ],
      }),
    ).toThrow('equipment assignment');
    expect(() =>
      decodeSessionView({
        ...SESSION_VIEW,
        party: [
          {
            ...member,
            feats: [{ ...feat, browserBonus: 2 }],
          },
        ],
      }),
    ).toThrow('missing or unknown');
  });

  it('closes tagged receipts and party-member selection facts', () => {
    expect(() =>
      decodeSessionView({
        ...SESSION_VIEW,
        latestReceipts: [{ ...OPPOSITION_ATTACK, browserTarget: 102 }],
      }),
    ).toThrow('missing or unknown');
    expect(() =>
      decodeSessionView({
        ...SESSION_VIEW,
        latestReceipts: [
          {
            ...OPPOSITION_ATTACK,
            target: {
              ...OPPOSITION_ATTACK.target,
              selectionPolicy: 'browser-choice',
            },
          },
        ],
      }),
    ).toThrow('selection policy');
    expect(() =>
      decodeSessionView({
        ...SESSION_VIEW,
        latestReceipts: [
          {
            ...OPPOSITION_ATTACK,
            target: {
              ...OPPOSITION_ATTACK.target,
              eligibleMemberCount: 0,
            },
          },
        ],
      }),
    ).toThrow('eligible party member count');
  });

  it('requires canonical Rust log identities and exact latest receipts', () => {
    expect(() =>
      decodeSessionView({
        ...SESSION_VIEW,
        log: [
          {
            ...SESSION_VIEW.log[0],
            id: 2,
          },
        ],
      }),
    ).toThrow('log identities');
    expect(() =>
      decodeSessionView({
        ...SESSION_VIEW,
        log: [],
      }),
    ).toThrow('latest receipts');
    expect(() =>
      decodeSessionView({
        ...SESSION_VIEW,
        log: [
          {
            ...SESSION_VIEW.log[0],
            browserAuthority: true,
          },
        ],
      }),
    ).toThrow('missing or unknown');
  });

  it('rejects forged roll arithmetic, damage, and activation lifecycle', () => {
    expect(() =>
      decodeSessionView({
        ...SESSION_VIEW,
        latestReceipts: [{ ...OPPOSITION_ATTACK, attackTotal: 18 }],
      }),
    ).toThrow('inconsistent arithmetic');
    expect(() =>
      decodeSessionView({
        ...SESSION_VIEW,
        latestReceipts: [
          { ...OPPOSITION_ATTACK, requestedDamage: 4, appliedDamage: 5 },
        ],
      }),
    ).toThrow('more damage');
    expect(() =>
      decodeSessionView({
        ...SESSION_VIEW,
        current: { ...ACTIVATION, entityId: 999 },
      }),
    ).toThrow('absent from the activation order');
    expect(() =>
      decodeSessionView({
        ...SESSION_VIEW,
        outcome: 'victory',
      }),
    ).toThrow('terminal session');
  });

  it('rejects browser-invented legal targets and inconsistent party status', () => {
    expect(() =>
      decodeSessionView({
        ...SESSION_VIEW,
        decision: {
          ...SESSION_VIEW.decision,
          actions: [
            {
              ...SESSION_VIEW.decision.actions[0],
              legalTargetEntityIds: [999],
            },
          ],
        },
      }),
    ).toThrow('nonvisible target');
    expect(() =>
      decodeSessionView({
        ...SESSION_VIEW,
        party: [
          {
            ...SESSION_VIEW.party[0],
            currentVitality: 0,
            conscious: true,
          },
        ],
      }),
    ).toThrow('consciousness');
    expect(() =>
      decodeSessionView({
        ...SESSION_VIEW,
        order: [{ ...ACTIVATION, side: 'opposition' }],
        current: { ...ACTIVATION, side: 'opposition' },
      }),
    ).toThrow('party identity');
    expect(() =>
      decodeSessionView({
        ...SESSION_VIEW,
        latestReceipts: [
          {
            ...OPPOSITION_ATTACK,
            target: {
              ...OPPOSITION_ATTACK.target,
              selectedMemberEntityId: 999,
            },
          },
        ],
      }),
    ).toThrow('party identity');
  });
});
