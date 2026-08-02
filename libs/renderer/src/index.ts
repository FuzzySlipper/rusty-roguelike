import {
  ChangeDetectionStrategy,
  Component,
  effect,
  input,
  output,
  signal,
  viewChild,
  type AfterViewInit,
  type ElementRef,
  type OnDestroy,
} from '@angular/core';
import type {
  RendererCameraSnapshot,
  RendererSurface,
} from '@rusty-engine/renderer-host';
import type { RenderHandle } from '@rusty-engine/render-contracts';

import {
  browserDevicePixelRatio,
  browserNow,
  browserPrefersReducedMotion,
  cancelBrowserFrame,
  loadBrowserBinaryAsset,
  observeElementSize,
  requestBrowserFrame,
} from '@rusty-roguelike/platform';
import type { SessionView } from '@rusty-roguelike/protocol';

import {
  cameraMotionCue,
  createDungeonFrame,
  TORCH_ASSET_ID,
  TORCH_CONTENT_HASH,
  type CameraMotionCue,
} from './dungeon-frame';

export {
  cameraMotionCue,
  createDungeonFrame,
  type CameraMotionCue,
  type DungeonFrame,
} from './dungeon-frame';

const CAMERA_POSITION = [0, 1.65, 0] as const;
const CAMERA_BASIS = {
  forward: [0, 0, -1] as const,
  right: [1, 0, 0] as const,
  up: [0, 1, 0] as const,
};
const CAMERA_PROJECTION = { fovYDegrees: 62, near: 0.08, far: 48 } as const;
const MOTION_DURATION_MS = 96;

@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  selector: 'rr-game-viewport',
  standalone: true,
  styles: [
    `
      :host,
      .viewport,
      canvas {
        display: block;
        height: 100%;
        inset: 0;
        position: absolute;
        width: 100%;
      }

      .viewport {
        background: #18130e;
        overflow: hidden;
      }

      canvas {
        cursor: crosshair;
        touch-action: manipulation;
      }

      .viewport::after {
        background: repeating-linear-gradient(
          0deg,
          transparent 0 3px,
          rgb(255 255 255 / 0.008) 3px 4px
        );
        content: '';
        inset: 0;
        pointer-events: none;
        position: absolute;
      }

      .reticle {
        border: 1px solid rgb(238 216 145 / 0.64);
        border-radius: 50%;
        height: 18px;
        left: 50%;
        pointer-events: none;
        position: absolute;
        top: 48%;
        transform: translate(-50%, -50%);
        width: 18px;
      }

      .failure {
        background: rgb(72 17 20 / 0.94);
        border: 1px solid rgb(255 135 135 / 0.55);
        left: 50%;
        max-width: min(84vw, 560px);
        padding: 1rem;
        position: absolute;
        top: 50%;
        transform: translate(-50%, -50%);
        z-index: 2;
      }
    `,
  ],
  template: `
    <section
      class="viewport"
      role="img"
      [attr.aria-label]="sceneLabel()"
      data-renderer-backend="rusty-engine-three"
      [attr.data-renderer-status]="rendererStatus()"
      [attr.data-visible-torches]="visibleTorchCount()"
      [attr.data-visible-lights]="visibleLightCount()"
      [attr.data-scene-cells]="sceneCellCount()"
    >
      <canvas
        #canvas
        width="960"
        height="540"
        aria-hidden="true"
        (pointerup)="pickActor($event)"
      ></canvas>
      <span class="reticle" aria-hidden="true"></span>
      @if (rendererError(); as message) {
        <p class="failure" role="alert">{{ message }}</p>
      }
    </section>
  `,
})
export class GameViewportComponent implements AfterViewInit, OnDestroy {
  readonly session = input.required<SessionView>();
  readonly selectedActionId = input<string | null>(null);
  readonly actorPicked = output<number>();
  protected readonly rendererError = signal<string | null>(null);
  protected readonly rendererStatus = signal<'loading' | 'ready' | 'error'>(
    'loading',
  );
  protected readonly sceneLabel = signal('First-person generated dungeon');
  protected readonly visibleTorchCount = signal(0);
  protected readonly visibleLightCount = signal(0);
  protected readonly sceneCellCount = signal(0);
  private readonly canvas =
    viewChild.required<ElementRef<HTMLCanvasElement>>('canvas');
  private surface: RendererSurface | null = null;
  private stopResize: (() => void) | null = null;
  private animationFrame: number | null = null;
  private retainedHandles: readonly RenderHandle[] = [];
  private renderedRevision: number | null = null;
  private renderedSelection: string | null = null;
  private destroyed = false;
  private sampleTransition:
    | ((
        transition: {
          readonly from: RendererCameraSnapshot;
          readonly to: RendererCameraSnapshot;
          readonly durationMilliseconds: number;
          readonly easing: 'smoothStep';
        },
        elapsedMilliseconds: number,
      ) => RendererCameraSnapshot)
    | null = null;

  constructor() {
    effect(() => {
      const session = this.session();
      const selectedActionId = this.selectedActionId();
      if (
        this.surface !== null &&
        (session.revision !== this.renderedRevision ||
          selectedActionId !== this.renderedSelection)
      ) {
        this.publishSession(session, selectedActionId);
      }
    });
  }

  async ngAfterViewInit(): Promise<void> {
    try {
      const { mountRendererAnimatedMeshSurface, sampleCameraTransition } =
        await import('@rusty-engine/renderer-host');
      if (this.destroyed) {
        return;
      }
      this.sampleTransition = sampleCameraTransition;
      const canvas = this.canvas().nativeElement;
      const surface = await mountRendererAnimatedMeshSurface(canvas, {
        autoStart: true,
        clearColor: 0x18130e,
        animatedMeshManifest: {
          kind: 'rusty_renderer_animated_mesh_resources.v1',
          resources: [
            {
              asset: TORCH_ASSET_ID,
              contentHash: TORCH_CONTENT_HASH,
              clipIds: [],
            },
          ],
        },
        resolveAnimatedMeshResource: () =>
          loadBrowserBinaryAsset('/assets/torch/medieval-torch.glb'),
        frame: { schemaVersion: 1, ops: [] },
        pixelRatio: browserDevicePixelRatio(),
        projection: CAMERA_PROJECTION,
      });
      if (this.destroyed) {
        surface.dispose();
        return;
      }
      this.surface = surface;
      this.surface.setCameraPose({
        position: CAMERA_POSITION,
        pitchDegrees: 0,
        yawDegrees: 0,
      });
      const published = this.publishSession(
        this.session(),
        this.selectedActionId(),
      );
      this.stopResize = observeElementSize(canvas, () =>
        this.surface?.renderOnce(),
      );
      if (published) {
        this.rendererStatus.set('ready');
      }
    } catch (error) {
      this.rendererError.set(
        `Rusty Engine could not mount the retained scene: ${
          error instanceof Error ? error.message : String(error)
        }`,
      );
      this.rendererStatus.set('error');
    }
  }

  protected pickActor(event: PointerEvent): void {
    const surface = this.surface;
    if (surface === null) {
      return;
    }
    const canvas = this.canvas().nativeElement;
    const bounds = canvas.getBoundingClientRect();
    const point = [
      ((event.clientX - bounds.left) / bounds.width) * 2 - 1,
      1 - ((event.clientY - bounds.top) / bounds.height) * 2,
    ] as const;
    const receipt = surface.pick({
      filter: { layers: ['scene'], tags: ['enemy'] },
      ray: { kind: 'viewport', point },
    });
    const entity = receipt.hint?.sourceTrace?.entity;
    if (entity !== undefined) {
      this.actorPicked.emit(entity);
    }
  }

  ngOnDestroy(): void {
    this.destroyed = true;
    this.cancelMotion();
    this.stopResize?.();
    this.stopResize = null;
    this.surface?.dispose();
    this.surface = null;
  }

  private publishSession(
    session: SessionView,
    selectedActionId: string | null,
  ): boolean {
    const surface = this.surface;
    if (surface === null) {
      return false;
    }
    this.cancelMotion();
    const dungeon = createDungeonFrame(
      session,
      this.retainedHandles,
      selectedActionId,
    );
    const receipt = surface.applyFrame(dungeon.frame);
    if (!receipt.applied) {
      this.rendererError.set(
        `Rusty Engine rejected the retained scene: ${receipt.diagnostics
          .map((diagnostic) => diagnostic.message)
          .join('; ')}`,
      );
      this.rendererStatus.set('error');
      return false;
    }
    this.retainedHandles = dungeon.handles;
    this.renderedRevision = session.revision;
    this.renderedSelection = selectedActionId;
    this.sceneLabel.set(
      `First-person dungeon facing ${session.world.facing}; ${session.world.visibleActors.length} visible enemies`,
    );
    this.visibleTorchCount.set(
      session.world.scenePlacements.filter(
        (placement) => placement.content.kind === 'prop',
      ).length,
    );
    this.visibleLightCount.set(
      session.world.scenePlacements.filter(
        (placement) => placement.content.kind === 'point_light',
      ).length,
    );
    this.sceneCellCount.set(session.world.cells.length);
    this.rendererError.set(null);
    this.rendererStatus.set('ready');
    const cue = cameraMotionCue(session.latestReceipts);
    if (cue === null || browserPrefersReducedMotion()) {
      surface.setCameraPose({
        position: CAMERA_POSITION,
        pitchDegrees: 0,
        yawDegrees: 0,
      });
      surface.renderOnce();
      return true;
    }
    this.animateMotion(cue);
    return true;
  }

  private animateMotion(cue: CameraMotionCue): void {
    const surface = this.surface;
    const sample = this.sampleTransition;
    if (surface === null || sample === null) {
      return;
    }
    const canvas = this.canvas().nativeElement;
    const viewport = {
      height: Math.max(canvas.clientHeight, 1),
      width: Math.max(canvas.clientWidth, 1),
    };
    const snapshot = (
      position: readonly [number, number, number],
      yawDegrees: number,
    ): RendererCameraSnapshot => ({
      basis: CAMERA_BASIS,
      pose: { position, pitchDegrees: 0, yawDegrees },
      projection: CAMERA_PROJECTION,
      viewport,
    });
    const transition = {
      durationMilliseconds: MOTION_DURATION_MS,
      easing: 'smoothStep' as const,
      from: snapshot(
        [cue.lateral, CAMERA_POSITION[1], cue.depth],
        cue.yawDegrees,
      ),
      to: snapshot(CAMERA_POSITION, 0),
    };
    const started = browserNow();
    const tick = (now: number): void => {
      const elapsed = Math.min(now - started, MOTION_DURATION_MS);
      const sampled = sample(transition, elapsed);
      surface.setCameraPose(sampled.pose, sampled.basis);
      surface.renderOnce(now);
      if (elapsed < MOTION_DURATION_MS && this.surface === surface) {
        this.animationFrame = requestBrowserFrame(tick);
      } else {
        this.animationFrame = null;
      }
    };
    this.animationFrame = requestBrowserFrame(tick);
  }

  private cancelMotion(): void {
    if (this.animationFrame !== null) {
      cancelBrowserFrame(this.animationFrame);
      this.animationFrame = null;
    }
  }
}
