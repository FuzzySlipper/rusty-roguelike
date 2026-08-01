import {
  renderHandle,
  type RenderDiff,
  type RenderFrameDiff,
  type RenderHandle,
  type Vec4,
} from '@rusty-engine/render-contracts';

import type {
  RelativeStep,
  SessionView,
  TurnReceipt,
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
  ceiling: [0.055, 0.07, 0.075, 1] as Vec4,
  enemy: [0.55, 0.16, 0.105, 1] as Vec4,
  enemyHit: [1, 0.48, 0.12, 1] as Vec4,
  enemyTarget: [0.82, 0.58, 0.16, 1] as Vec4,
  floor: [0.09, 0.115, 0.105, 1] as Vec4,
  impact: [0.95, 0.18, 0.08, 1] as Vec4,
  wall: [0.18, 0.22, 0.205, 1] as Vec4,
};
const CELL_SIZE = 2.4;

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
  const createCuboid = (
    handleValue: number,
    label: string,
    translation: readonly [number, number, number],
    scale: readonly [number, number, number],
    color: Vec4,
    sourceEntity: number | null,
    tags: readonly string[],
    layer: 'scene' | 'viewmodel' = 'scene',
  ): void => {
    const handle = renderHandle(handleValue);
    handles.push(handle);
    ops.push({
      op: 'create',
      handle,
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
        layer,
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
        COLORS.floor,
        null,
        ['rusty-roguelike', 'dungeon-floor'],
      );
      createCuboid(
        base + 1,
        `ceiling-${cell.lateral}-${cell.depth}`,
        [x, 3.05, z],
        [CELL_SIZE - 0.06, 0.18, CELL_SIZE - 0.06],
        COLORS.ceiling,
        null,
        ['rusty-roguelike', 'dungeon-ceiling'],
      );
    } else {
      createCuboid(
        base + 2,
        `wall-${cell.lateral}-${cell.depth}`,
        [x, 1.48, z],
        [CELL_SIZE, 3.2, CELL_SIZE],
        COLORS.wall,
        null,
        ['rusty-roguelike', 'dungeon-wall'],
      );
    }
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
        ? COLORS.enemyHit
        : legalTargets.has(actor.entityId)
          ? COLORS.enemyTarget
          : COLORS.enemy,
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
        ? COLORS.enemyHit
        : legalTargets.has(actor.entityId)
          ? COLORS.enemyTarget
          : COLORS.enemy,
      actor.entityId,
      [
        'rusty-roguelike',
        'enemy',
        `enemy-${actor.entityId}`,
        ...(legalTargets.has(actor.entityId) ? ['legal-target'] : []),
      ],
    );
  }

  if (
    session.latestReceipts.some(
      (receipt) => receipt.kind === 'oppositionAttacked',
    )
  ) {
    createCuboid(
      9_001,
      `party-impact-${session.revision}`,
      [0, 0.2, -0.7],
      [1.35, 0.06, 0.18],
      COLORS.impact,
      null,
      ['rusty-roguelike', 'party-impact'],
      'viewmodel',
    );
  }

  return { frame: { schemaVersion: 1, ops }, handles };
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
