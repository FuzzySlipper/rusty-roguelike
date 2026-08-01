import {
  ChangeDetectionStrategy,
  Component,
  inject,
  type OnInit,
} from '@angular/core';

import { GameViewportComponent } from '@rusty-roguelike/renderer';
import { BootstrapStore } from '@rusty-roguelike/store';

@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [GameViewportComponent],
  selector: 'rr-game-shell',
  standalone: true,
  styles: [
    `
      :host,
      main {
        display: block;
        height: 100dvh;
        inset: 0;
        overflow: hidden;
        position: fixed;
        width: 100vw;
      }

      .overlay {
        background: var(--rr-panel);
        border: 1px solid var(--rr-line);
        border-radius: 12px;
        left: 50%;
        max-height: calc(100dvh - 2rem);
        max-width: min(700px, calc(100vw - 2rem));
        overflow: auto;
        padding: clamp(1rem, 3vw, 2rem);
        pointer-events: auto;
        position: absolute;
        top: 50%;
        transform: translate(-50%, -50%);
        width: 100%;
        z-index: 2;
      }

      .eyebrow,
      dt {
        color: var(--rr-accent);
        font-size: 0.75rem;
        font-weight: 700;
        letter-spacing: 0.12em;
        text-transform: uppercase;
      }

      h1 {
        font-size: clamp(2rem, 8vw, 4.4rem);
        line-height: 0.95;
        margin: 0.35rem 0 1rem;
      }

      p {
        color: var(--rr-muted);
        line-height: 1.55;
      }

      dl {
        display: grid;
        gap: 0.75rem;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        margin: 1.5rem 0 0;
      }

      dl > div {
        background: rgb(255 255 255 / 0.035);
        border: 1px solid var(--rr-line);
        min-width: 0;
        padding: 0.85rem;
      }

      dd {
        font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
        font-size: 0.78rem;
        margin: 0.35rem 0 0;
        overflow-wrap: anywhere;
      }

      .failure {
        color: var(--rr-danger);
      }

      @media (max-width: 560px) {
        .overlay {
          bottom: 0.75rem;
          left: 0.75rem;
          max-height: calc(100dvh - 1.5rem);
          max-width: calc(100vw - 1.5rem);
          padding: 1rem;
          top: auto;
          transform: none;
        }

        dl {
          grid-template-columns: 1fr;
        }
      }
    `,
  ],
  template: `
    <main>
      <rr-game-viewport />
      <section class="overlay" aria-labelledby="product-title">
        <span class="eyebrow">Rust-owned expedition</span>
        <h1 id="product-title">Rusty Roguelike</h1>
        @switch (store.state().status) {
          @case ('loading') {
            <p role="status">Linking the public Engine and Procgen runtimes…</p>
          }
          @case ('ready') {
            <p>
              The retained renderer and same-origin Rust host are ready. The
              first generated expedition arrives in the next reviewed slice.
            </p>
            @if (store.state(); as state) {
              @if (state.status === 'ready') {
                <dl aria-label="Exact dependency readout">
                  <div>
                    <dt>Rusty Engine</dt>
                    <dd data-testid="engine-revision">
                      {{ state.value.rustyEngineRevision }}
                    </dd>
                  </div>
                  <div>
                    <dt>Rusty Procgen</dt>
                    <dd data-testid="procgen-revision">
                      {{ state.value.rustyProcgenRevision }}
                    </dd>
                  </div>
                </dl>
              }
            }
          }
          @case ('error') {
            @if (store.state(); as state) {
              @if (state.status === 'error') {
                <p class="failure" role="alert">{{ state.message }}</p>
              }
            }
          }
        }
      </section>
    </main>
  `,
})
export class GameShellComponent implements OnInit {
  protected readonly store = inject(BootstrapStore);

  ngOnInit(): void {
    void this.store.load();
  }
}
