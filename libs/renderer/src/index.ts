import {
  ChangeDetectionStrategy,
  Component,
  signal,
  viewChild,
  type AfterViewInit,
  type ElementRef,
  type OnDestroy,
} from '@angular/core';
import type { RendererSurface } from '@rusty-engine/renderer-host';

import {
  browserDevicePixelRatio,
  observeElementSize,
} from '@rusty-roguelike/platform';

import { createBootstrapFrame } from './bootstrap-frame';

export { createBootstrapFrame, type BootstrapFrame } from './bootstrap-frame';

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
        background: #05090c;
        overflow: hidden;
      }

      .viewport::after {
        background:
          linear-gradient(180deg, rgb(0 0 0 / 0.02), rgb(0 0 0 / 0.35)),
          repeating-linear-gradient(
            0deg,
            transparent 0 3px,
            rgb(255 255 255 / 0.012) 3px 4px
          );
        content: '';
        inset: 0;
        pointer-events: none;
        position: absolute;
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
      aria-label="Rusty Engine retained bootstrap scene"
      data-renderer-backend="rusty-engine-three"
      [attr.data-renderer-status]="rendererStatus()"
    >
      <canvas #canvas width="960" height="540" aria-hidden="true"></canvas>
      @if (rendererError(); as message) {
        <p class="failure" role="alert">{{ message }}</p>
      }
    </section>
  `,
})
export class GameViewportComponent implements AfterViewInit, OnDestroy {
  protected readonly rendererError = signal<string | null>(null);
  protected readonly rendererStatus = signal<'loading' | 'ready' | 'error'>(
    'loading',
  );
  private readonly canvas =
    viewChild.required<ElementRef<HTMLCanvasElement>>('canvas');
  private surface: RendererSurface | null = null;
  private stopResize: (() => void) | null = null;
  private destroyed = false;

  async ngAfterViewInit(): Promise<void> {
    try {
      const { mountRendererSurface } = await import(
        '@rusty-engine/renderer-host'
      );
      if (this.destroyed) {
        return;
      }
      const canvas = this.canvas().nativeElement;
      this.surface = mountRendererSurface(canvas, {
        autoStart: true,
        clearColor: 0x05090c,
        frame: createBootstrapFrame().frame,
        pixelRatio: browserDevicePixelRatio(),
        projection: { fovYDegrees: 58, near: 0.1, far: 64 },
      });
      this.surface.setCameraPose({
        position: [0, 2.1, 4.8],
        pitchDegrees: -8,
        yawDegrees: 0,
      });
      this.surface.renderOnce();
      this.stopResize = observeElementSize(canvas, () =>
        this.surface?.renderOnce(),
      );
      this.rendererStatus.set('ready');
    } catch (error) {
      this.rendererError.set(
        `Rusty Engine could not mount the retained scene: ${
          error instanceof Error ? error.message : String(error)
        }`,
      );
      this.rendererStatus.set('error');
    }
  }

  ngOnDestroy(): void {
    this.destroyed = true;
    this.stopResize?.();
    this.stopResize = null;
    this.surface?.dispose();
    this.surface = null;
  }
}
