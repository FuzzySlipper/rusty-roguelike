import {
  renderHandle,
  type RenderDiff,
  type RenderFrameDiff,
  type Vec4,
} from '@rusty-engine/render-contracts';

export interface BootstrapFrame {
  readonly frame: RenderFrameDiff;
}

const COLORS = {
  floor: [0.035, 0.055, 0.065, 1] as Vec4,
  horizon: [0.055, 0.095, 0.105, 1] as Vec4,
  accent: [0.2, 0.66, 0.58, 1] as Vec4,
};

export function createBootstrapFrame(): BootstrapFrame {
  const ops: RenderDiff[] = [];
  let nextHandle = 100;

  const addCuboid = (
    label: string,
    translation: readonly [number, number, number],
    scale: readonly [number, number, number],
    color: Vec4,
  ): void => {
    ops.push({
      op: 'create',
      handle: renderHandle(nextHandle++),
      parent: null,
      node: {
        geometry: { kind: 'cube' },
        material: { color, wireframe: false },
        transform: {
          translation,
          rotation: [0, 0, 0, 1],
          scale,
        },
        visible: true,
        layer: 'scene',
        metadata: {
          sourceEntity: null,
          sourceSceneNode: null,
          tags: ['rusty-roguelike', 'bootstrap-backdrop'],
          label: `bootstrap-${label}`,
        },
      },
    });
  };

  addCuboid('floor', [0, -0.15, -5], [16, 0.3, 18], COLORS.floor);
  addCuboid('horizon', [0, 3.2, -13.5], [18, 6.5, 0.4], COLORS.horizon);
  addCuboid('left-marker', [-5.2, 1.4, -7], [0.45, 3.1, 0.45], COLORS.accent);
  addCuboid('right-marker', [5.2, 1.4, -7], [0.45, 3.1, 0.45], COLORS.accent);
  addCuboid('threshold', [0, 0.12, -6.2], [5.6, 0.24, 3.6], COLORS.accent);

  return { frame: { schemaVersion: 1, ops } };
}
