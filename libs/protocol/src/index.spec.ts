import { describe, expect, it } from 'vitest';

import {
  decodeBootstrapReadout,
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
  schemaVersion: 1,
  revision: 17,
  floorId: 'floor.occlusion-regression',
  facing: 'north',
  discoveredCellCount: 8,
  cells: [
    { lateral: 0, depth: 0, kind: 'floor' },
    { lateral: 0, depth: 1, kind: 'floor' },
    { lateral: 0, depth: 2, kind: 'wall' },
  ],
  visibleActors: [],
};

describe('decodeWorldView', () => {
  it('accepts the bounded relative Rust projection', () => {
    expect(decodeWorldView(WORLD_VIEW)).toEqual(WORLD_VIEW);
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
  });
});
