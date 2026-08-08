import {
  ChangeDetectionStrategy,
  Component,
  computed,
  input,
} from '@angular/core';

import type { SessionView } from '@rusty-roguelike/protocol';

/**
 * Browser-shell marker for the native renderer mount region. The Engine-owned
 * renderer is mounted only by the Rust product host; this component neither
 * imports renderer packages nor creates a browser rendering surface.
 */
@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  selector: 'rr-native-viewport-boundary',
  standalone: true,
  styles: [
    `
      :host,
      .viewport-boundary {
        display: block;
        height: 100%;
        inset: 0;
        position: absolute;
        width: 100%;
      }

      .viewport-boundary {
        align-content: center;
        background:
          linear-gradient(rgb(10 18 18 / 0.58), rgb(10 18 18 / 0.92)),
          radial-gradient(circle at 50% 45%, #31504a, #18130e 68%);
        color: var(--rr-muted);
        display: grid;
        justify-items: center;
        padding: 2rem;
        text-align: center;
      }

      strong {
        color: var(--rr-text);
        display: block;
        font-size: clamp(1rem, 2vw, 1.35rem);
        margin-bottom: 0.45rem;
      }

      p {
        margin: 0;
        max-width: 38rem;
      }
    `,
  ],
  template: `
    <section
      class="viewport-boundary"
      role="img"
      [attr.aria-label]="sceneLabel()"
      data-renderer-backend="rusty-engine-native-webview"
      data-renderer-owner="rust"
      data-renderer-status="native-host"
    >
      <div>
        <strong>Native Engine viewport</strong>
        <p>
          Launch <code>pnpm run native</code> for the Engine-owned retained
          renderer. This browser surface remains the gameplay and accessibility
          control shell only.
        </p>
      </div>
    </section>
  `,
})
export class NativeViewportBoundaryComponent {
  readonly session = input.required<SessionView>();
  protected readonly sceneLabel = computed(() => {
    const view = this.session();
    return `Native first-person dungeon facing ${view.world.facing}; ${view.world.visibleActors.length} visible enemies`;
  });
}
