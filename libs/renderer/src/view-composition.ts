import type { RendererViewComposition } from '@rusty-engine/render-contracts';

export const DUNGEON_VIEW_CAMERA_ID = 'camera.dungeon-local-overview';
export const DUNGEON_VIEW_TARGET_ID = 'target.dungeon-local-overview';

/**
 * Presentation-only view of the already admitted retained scene. It receives
 * no discovery, visibility, navigation, or gameplay facts.
 */
export function createDungeonViewComposition(
  targetRevision: number,
  compact: boolean,
): RendererViewComposition {
  const targetSize = compact ? 128 : 256;
  return {
    schemaVersion: 1,
    cameras: [
      {
        id: DUNGEON_VIEW_CAMERA_ID,
        pose: { position: [0, 15, 0], pitchDegrees: -90, yawDegrees: 0 },
        projection: {
          kind: 'orthographic',
          verticalSize: 22,
          near: 0.1,
          far: 32,
        },
      },
    ],
    targets: [
      {
        id: DUNGEON_VIEW_TARGET_ID,
        revision: targetRevision,
        width: targetSize,
        height: targetSize,
        color: 'rgba8_srgb',
        depth: 'depth24',
        sampling: 'nearest',
      },
    ],
    views: [
      {
        id: 'view.dungeon-local-overview',
        cameraId: DUNGEON_VIEW_CAMERA_ID,
        target: {
          kind: 'offscreen',
          targetId: DUNGEON_VIEW_TARGET_ID,
          targetRevision,
        },
        viewport: { x: 0, y: 0, width: 1, height: 1 },
        order: 10,
      },
    ],
    presentations: [
      {
        id: 'presentation.dungeon-local-overview',
        sourceTargetId: DUNGEON_VIEW_TARGET_ID,
        sourceTargetRevision: targetRevision,
        destination: {
          kind: 'primary',
          viewport: compact
            ? { x: 0.66, y: 0.7, width: 0.3, height: 0.26 }
            : { x: 0.68, y: 0.62, width: 0.28, height: 0.32 },
        },
        order: 20,
      },
    ],
  };
}

export function compactDungeonView(width: number): boolean {
  return width < 640;
}
