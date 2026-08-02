import { describe, expect, it } from 'vitest';

import type { MinimapView } from '@rusty-roguelike/protocol';

import {
  facingRotation,
  featureGlyph,
  layoutMinimap,
  minimapDetail,
} from './minimap';

const MINIMAP: MinimapView = {
  party: { x: -2, y: 4 },
  facing: 'east',
  cells: [
    {
      x: -2,
      y: 4,
      terrain: 'floor',
      feature: 'entry',
      visible: true,
    },
    {
      x: -1,
      y: 4,
      terrain: 'floor',
      feature: 'locked-door',
      visible: true,
    },
    {
      x: 0,
      y: 4,
      terrain: 'wall',
      feature: null,
      visible: false,
    },
  ],
  visibleActors: [
    {
      actorId: 'ember-rat',
      entityId: 101,
      name: 'Ember Rat',
      x: -1,
      y: 4,
      participating: true,
    },
  ],
};

describe('minimap presentation', () => {
  it('maps every strict Rust fact into one translated display frame', () => {
    const layout = layoutMinimap(MINIMAP);

    expect(layout).toMatchObject({
      width: 5,
      height: 3,
      partyX: 1,
      partyY: 1,
    });
    expect(layout.cells).toEqual([
      { ...MINIMAP.cells[0], key: '-2,4', mapX: 1, mapY: 1 },
      { ...MINIMAP.cells[1], key: '-1,4', mapX: 2, mapY: 1 },
      { ...MINIMAP.cells[2], key: '0,4', mapX: 3, mapY: 1 },
    ]);
    expect(layout.visibleActors).toEqual([
      {
        entityId: 101,
        mapX: 2,
        mapY: 1,
        name: 'Ember Rat',
        participating: true,
      },
    ]);
  });

  it('renders authored facing and feature kinds without gameplay inference', () => {
    expect(
      (['north', 'east', 'south', 'west'] as const).map(facingRotation),
    ).toEqual([0, 90, 180, 270]);
    expect(
      (
        ['entry', 'gate', 'goal', 'key', 'open-door', 'locked-door'] as const
      ).map(featureGlyph),
    ).toEqual(['⌂', '▣', '◆', '⚿', '/', '+']);
  });

  it('provides the complete projected map as a text alternative', () => {
    expect(minimapDetail(MINIMAP)).toBe(
      'Party at -2, 4. visible floor with entry at -2, 4 visible floor with locked-door at -1, 4 remembered wall at 0, 4 Ember Rat, participating, at -1, 4',
    );
  });
});
