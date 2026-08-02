import type { RendererViewComposition } from '@rusty-engine/render-contracts';

export const DUNGEON_VIEW_CAMERA_ID = 'camera.dungeon-local-overview';
export const DUNGEON_VIEW_TARGET_ID = 'target.dungeon-local-overview';

/**
 * Offscreen lookup view of the already admitted retained scene. It receives
 * no discovery, visibility, navigation, or gameplay facts and is deliberately
 * not presented into the primary canvas; the detailed Rust minimap owns the
 * visible map presentation.
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
    presentations: [],
  };
}

export function compactDungeonView(width: number): boolean {
  return width < 640;
}
