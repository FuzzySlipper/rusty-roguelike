import { describe, expect, it } from 'vitest';

import { createBootstrapFrame } from './bootstrap-frame';

describe('createBootstrapFrame', () => {
  it('contains only identity-free abstract bootstrap nodes', () => {
    const scene = createBootstrapFrame();
    expect(scene.frame.ops).toHaveLength(5);
    for (const operation of scene.frame.ops) {
      expect(operation.op).toBe('create');
      if (operation.op === 'create') {
        expect(operation.node.metadata.sourceEntity).toBeNull();
        expect(operation.node.metadata.sourceSceneNode).toBeNull();
        expect(operation.node.metadata.tags).toEqual([
          'rusty-roguelike',
          'bootstrap-backdrop',
        ]);
      }
    }
  });
});
