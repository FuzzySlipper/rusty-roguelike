import { describe, expect, it } from 'vitest';

import { validateRendererViewComposition } from '@rusty-engine/render-contracts';

import {
  compactDungeonView,
  createDungeonViewComposition,
  DUNGEON_VIEW_CAMERA_ID,
  DUNGEON_VIEW_TARGET_ID,
} from './view-composition';

describe('dungeon offscreen lookup composition', () => {
  it('retains only bounded renderer facts without a primary presentation', () => {
    const composition = validateRendererViewComposition(
      createDungeonViewComposition(7, false),
    );

    expect(composition.cameras.map(({ id }) => id)).toEqual([
      DUNGEON_VIEW_CAMERA_ID,
    ]);
    expect(composition.targets).toEqual([
      expect.objectContaining({
        id: DUNGEON_VIEW_TARGET_ID,
        revision: 7,
        width: 256,
        height: 256,
      }),
    ]);
    expect(composition.views[0]?.target).toEqual({
      kind: 'offscreen',
      targetId: DUNGEON_VIEW_TARGET_ID,
      targetRevision: 7,
    });
    expect(composition.presentations).toEqual([]);
    expect(JSON.stringify(composition)).not.toMatch(
      /discover|visible|actor|navigation|legal/iu,
    );
  });

  it('selects a smaller target and bounded inset for compact canvases', () => {
    expect(compactDungeonView(639)).toBe(true);
    expect(compactDungeonView(640)).toBe(false);
    const compact = validateRendererViewComposition(
      createDungeonViewComposition(8, true),
    );
    expect(compact.targets[0]?.width).toBe(128);
    expect(compact.presentations).toEqual([]);
  });
});
