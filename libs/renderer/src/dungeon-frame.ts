import {
  renderHandle,
  type RenderDiff,
  type RenderFrameDiff,
  type RenderHandle,
  type RenderMaterialDescriptor,
  type StaticMeshAsset,
  type Transform,
  type Vec4,
} from '@rusty-engine/render-contracts';

import type {
  RelativeStep,
  RelativeSceneFacing,
  SessionView,
  TurnReceipt,
  VisibleScenePlacementView,
} from '@rusty-roguelike/protocol';

export interface DungeonFrame {
  readonly frame: RenderFrameDiff;
  readonly handles: readonly RenderHandle[];
}

export interface CameraMotionCue {
  readonly kind: 'step' | 'turn';
  readonly lateral: number;
  readonly depth: number;
  readonly yawDegrees: number;
}

const COLORS = {
  ceiling: [0.21, 0.2, 0.17, 1] as Vec4,
  enemy: [0.55, 0.16, 0.105, 1] as Vec4,
  enemyHit: [1, 0.48, 0.12, 1] as Vec4,
  enemyTarget: [0.82, 0.58, 0.16, 1] as Vec4,
  floor: [0.28, 0.265, 0.225, 1] as Vec4,
  wall: [0.4, 0.375, 0.315, 1] as Vec4,
};
const CELL_SIZE = 2.4;
const CUBE_ASSET_ID = 'asset.mesh.dungeon-cube';
const MATERIAL_IDS = {
  ceiling: 'material.dungeon.ceiling',
  enemy: 'material.dungeon.enemy',
  enemyHit: 'material.dungeon.enemy-hit',
  enemyTarget: 'material.dungeon.enemy-target',
  floor: 'material.dungeon.floor',
  wall: 'material.dungeon.wall',
} as const;
const DUNGEON_MATERIALS = Object.entries(MATERIAL_IDS).map(
  ([kind, id]): RenderMaterialDescriptor => ({
    schemaVersion: 2,
    id,
    color: COLORS[kind as keyof typeof COLORS],
    texture: null,
    roughness: 0.92,
    textureTint: [1, 1, 1, 1],
    emissionColor: [0, 0, 0],
    emissionIntensity: 0,
    uvStrategy: 'flat',
  }),
);
const DUNGEON_CUBE: StaticMeshAsset = {
  asset: CUBE_ASSET_ID,
  payload: {
    layout: {
      vertexCount: 24,
      indexCount: 36,
      indexWidth: 'u32',
      attributes: [
        { name: 'position', components: 3, kind: 'f32' },
        { name: 'normal', components: 3, kind: 'f32' },
      ],
    },
    groups: [{ materialSlot: 0, start: 0, count: 36 }],
    bounds: { min: [-0.5, -0.5, -0.5], max: [0.5, 0.5, 0.5] },
    source: {
      kind: 'inline',
      positions: [
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.5, 0.5, 0.5],
        [-0.5, 0.5, 0.5],
        [0.5, -0.5, -0.5],
        [-0.5, -0.5, -0.5],
        [-0.5, 0.5, -0.5],
        [0.5, 0.5, -0.5],
        [-0.5, 0.5, 0.5],
        [0.5, 0.5, 0.5],
        [0.5, 0.5, -0.5],
        [-0.5, 0.5, -0.5],
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, -0.5, 0.5],
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
        [0.5, 0.5, 0.5],
        [-0.5, -0.5, -0.5],
        [-0.5, -0.5, 0.5],
        [-0.5, 0.5, 0.5],
        [-0.5, 0.5, -0.5],
      ].flat(),
      normals: [
        [0, 0, 1],
        [0, 0, 1],
        [0, 0, 1],
        [0, 0, 1],
        [0, 0, -1],
        [0, 0, -1],
        [0, 0, -1],
        [0, 0, -1],
        [0, 1, 0],
        [0, 1, 0],
        [0, 1, 0],
        [0, 1, 0],
        [0, -1, 0],
        [0, -1, 0],
        [0, -1, 0],
        [0, -1, 0],
        [1, 0, 0],
        [1, 0, 0],
        [1, 0, 0],
        [1, 0, 0],
        [-1, 0, 0],
        [-1, 0, 0],
        [-1, 0, 0],
        [-1, 0, 0],
      ].flat(),
      indices: [
        0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 8, 9, 10, 8, 10, 11, 12, 13, 14, 12,
        14, 15, 16, 17, 18, 16, 18, 19, 20, 21, 22, 20, 22, 23,
      ],
    },
    provenance: 'generated',
  },
  materialSlots: [{ slot: 0, material: MATERIAL_IDS.wall }],
  collision: { kind: 'visualOnly' },
};
export const TORCH_ASSET_ID = 'asset.prop.torch.medieval';
export const TORCH_CONTENT_HASH =
  'sha256:49d74d297a4b7b8a271ad1299ea3a16608cb4cc460e0ea1d5a2ede36a13b5a2e';

export function createDungeonFrame(
  session: SessionView,
  previousHandles: readonly RenderHandle[] = [],
  selectedActionId: string | null = null,
): DungeonFrame {
  const ops: RenderDiff[] = previousHandles.map((handle) => ({
    handle,
    op: 'destroy',
  }));
  const handles: RenderHandle[] = [];
  if (previousHandles.length === 0) {
    ops.push(
      ...DUNGEON_MATERIALS.map(
        (material): RenderDiff => ({ op: 'defineMaterial', material }),
      ),
    );
  }
  // Engine releases a static asset when its last retained instance is destroyed.
  // Re-admit the shared cube after the prior handle set is removed, before the
  // replacement instances borrow it for this complete scene publication.
  ops.push({ op: 'defineStaticMesh', asset: DUNGEON_CUBE });
  const ambientHandle = renderHandle(50);
  handles.push(ambientHandle);
  ops.push({
    op: 'createLight',
    handle: ambientHandle,
    parent: null,
    light: {
      kind: 'ambient',
      color: [0.92, 0.88, 0.76],
      intensity: 0.62,
      enabled: true,
      shadowIntent: 'disabled',
    },
  });
  const directionalHandle = renderHandle(51);
  handles.push(directionalHandle);
  ops.push({
    op: 'createLight',
    handle: directionalHandle,
    parent: null,
    light: {
      kind: 'directional',
      color: [0.86, 0.88, 0.82],
      intensity: 0.7,
      enabled: true,
      direction: [-0.35, -1, -0.45],
      shadowIntent: 'requested',
    },
  });
  const createCuboid = (
    handleValue: number,
    label: string,
    translation: readonly [number, number, number],
    scale: readonly [number, number, number],
    material: string,
    sourceEntity: number | null,
    tags: readonly string[],
  ): void => {
    const handle = renderHandle(handleValue);
    handles.push(handle);
    ops.push({
      op: 'createStaticMeshInstance',
      handle,
      parent: null,
      instance: {
        asset: CUBE_ASSET_ID,
        transform: {
          translation,
          rotation: [0, 0, 0, 1],
          scale,
        },
        visible: true,
        materialOverrides: [{ slot: 0, material }],
        metadata: {
          sourceEntity,
          sourceSceneNode: null,
          tags,
          label,
        },
      },
    });
  };

  for (const cell of session.world.cells) {
    const x = cell.lateral * CELL_SIZE;
    const z = -cell.depth * CELL_SIZE;
    const cellIndex = cell.depth * 13 + cell.lateral + 6;
    const base = 100 + cellIndex * 4;
    if (cell.kind === 'floor') {
      createCuboid(
        base,
        `floor-${cell.lateral}-${cell.depth}`,
        [x, -0.12, z],
        [CELL_SIZE - 0.06, 0.24, CELL_SIZE - 0.06],
        MATERIAL_IDS.floor,
        null,
        ['rusty-roguelike', 'dungeon-floor'],
      );
      createCuboid(
        base + 1,
        `ceiling-${cell.lateral}-${cell.depth}`,
        [x, 3.05, z],
        [CELL_SIZE - 0.06, 0.18, CELL_SIZE - 0.06],
        MATERIAL_IDS.ceiling,
        null,
        ['rusty-roguelike', 'dungeon-ceiling'],
      );
    } else {
      createCuboid(
        base + 2,
        `wall-${cell.lateral}-${cell.depth}`,
        [x, 1.48, z],
        [CELL_SIZE, 3.2, CELL_SIZE],
        MATERIAL_IDS.wall,
        null,
        ['rusty-roguelike', 'dungeon-wall'],
      );
    }
  }

  const visibleProps = session.world.scenePlacements.filter(
    (placement) => placement.content.kind === 'prop',
  );
  if (visibleProps.length > 0) {
    ops.push({
      op: 'defineAnimatedMesh',
      asset: {
        asset: TORCH_ASSET_ID,
        runtimeFormat: 'glb',
        contentHash: TORCH_CONTENT_HASH,
        clips: [],
        defaultClip: null,
        materialSlots: [],
        bounds: { min: [-0.254, -1, -0.182], max: [0.254, 1, 0.182] },
      },
    });
  }
  for (const [index, placement] of session.world.scenePlacements.entries()) {
    const transform = sceneTransform(placement);
    if (placement.content.kind === 'prop') {
      const handle = renderHandle(30_000 + index);
      handles.push(handle);
      ops.push({
        op: 'createAnimatedMeshInstance',
        handle,
        parent: null,
        instance: {
          asset: TORCH_ASSET_ID,
          transform,
          visible: true,
          materialOverrides: [],
          playback: null,
          metadata: {
            sourceEntity: null,
            sourceSceneNode: null,
            tags: ['rusty-roguelike', 'dungeon-prop', 'torch'],
            label: placement.id,
          },
        },
      });
      continue;
    }
    const handle = renderHandle(20_000 + index);
    handles.push(handle);
    ops.push({
      op: 'createLight',
      handle,
      parent: null,
      light: {
        kind: 'point',
        color: rgbColor(placement.content.colorRgb),
        intensity: placement.content.intensityMilli / 1_000,
        enabled: true,
        position: [transform.translation[0], 2.05, transform.translation[2]],
        range: placement.content.rangeCells * CELL_SIZE,
        decay: 2,
        shadowIntent: 'requested',
      },
    });
  }

  const hitTarget = latestPartyAttackTarget(session.latestReceipts);
  const legalTargets = new Set(
    session.decision?.actions.find(
      (action) => action.actionId === selectedActionId,
    )?.legalTargetEntityIds ?? [],
  );
  for (const actor of session.world.visibleActors) {
    const x = actor.lateral * CELL_SIZE;
    const z = -actor.depth * CELL_SIZE;
    const base = 10_000 + actor.entityId * 2;
    createCuboid(
      base,
      `enemy-${actor.entityId}`,
      [x, 0.92, z],
      [0.9, 1.75, 0.58],
      actor.entityId === hitTarget
        ? MATERIAL_IDS.enemyHit
        : legalTargets.has(actor.entityId)
          ? MATERIAL_IDS.enemyTarget
          : MATERIAL_IDS.enemy,
      actor.entityId,
      [
        'rusty-roguelike',
        'enemy',
        `enemy-${actor.entityId}`,
        ...(legalTargets.has(actor.entityId) ? ['legal-target'] : []),
      ],
    );
    createCuboid(
      base + 1,
      `enemy-head-${actor.entityId}`,
      [x, 2.03, z],
      [0.62, 0.62, 0.62],
      actor.entityId === hitTarget
        ? MATERIAL_IDS.enemyHit
        : legalTargets.has(actor.entityId)
          ? MATERIAL_IDS.enemyTarget
          : MATERIAL_IDS.enemy,
      actor.entityId,
      [
        'rusty-roguelike',
        'enemy',
        `enemy-${actor.entityId}`,
        ...(legalTargets.has(actor.entityId) ? ['legal-target'] : []),
      ],
    );
  }

  return { frame: { schemaVersion: 1, ops }, handles };
}

function sceneTransform(placement: VisibleScenePlacementView): Transform {
  const offset = facingOffset(placement.facing);
  return {
    translation: [
      placement.lateral * CELL_SIZE + offset[0],
      1.48,
      -placement.depth * CELL_SIZE + offset[1],
    ],
    rotation: facingRotation(placement.facing),
    scale: [0.72, 0.72, 0.72],
  };
}

function facingOffset(facing: RelativeSceneFacing): readonly [number, number] {
  const offsets: Record<RelativeSceneFacing, readonly [number, number]> = {
    forward: [0, -0.92],
    right: [0.92, 0],
    backward: [0, 0.92],
    left: [-0.92, 0],
  };
  return offsets[facing];
}

function facingRotation(
  facing: RelativeSceneFacing,
): readonly [number, number, number, number] {
  const yaw: Record<RelativeSceneFacing, number> = {
    forward: 0,
    right: -Math.PI / 2,
    backward: Math.PI,
    left: Math.PI / 2,
  };
  const half = yaw[facing] / 2;
  return [0, Math.sin(half), 0, Math.cos(half)];
}

function rgbColor(value: string): readonly [number, number, number] {
  return [
    Number.parseInt(value.slice(1, 3), 16) / 255,
    Number.parseInt(value.slice(3, 5), 16) / 255,
    Number.parseInt(value.slice(5, 7), 16) / 255,
  ];
}

export function cameraMotionCue(
  receipts: readonly TurnReceipt[],
): CameraMotionCue | null {
  const motion = receipts.find(
    (receipt) =>
      receipt.kind === 'partyMoved' || receipt.kind === 'partyTurned',
  );
  if (motion?.kind === 'partyMoved') {
    return stepCue(motion.step);
  }
  if (motion?.kind === 'partyTurned') {
    return {
      kind: 'turn',
      lateral: 0,
      depth: 0,
      yawDegrees: motion.direction === 'left' ? 90 : -90,
    };
  }
  return null;
}

function stepCue(step: RelativeStep): CameraMotionCue {
  const offset: Record<RelativeStep, readonly [number, number]> = {
    backward: [0, -0.72],
    forward: [0, 0.72],
    left: [-0.72, 0],
    right: [0.72, 0],
  };
  return {
    kind: 'step',
    lateral: offset[step][0],
    depth: offset[step][1],
    yawDegrees: 0,
  };
}

function latestPartyAttackTarget(
  receipts: readonly TurnReceipt[],
): number | null {
  return (
    receipts.find((receipt) => receipt.kind === 'partyAttacked')
      ?.targetEntityId ?? null
  );
}
