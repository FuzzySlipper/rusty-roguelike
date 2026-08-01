import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  inject,
  input,
  signal,
} from '@angular/core';

import type { LoadoutDragPayload } from '@rusty-roguelike/platform';
import type { SessionView } from '@rusty-roguelike/protocol';
import { SessionStore } from '@rusty-roguelike/store';

import { LoadoutPanelComponent, type LoadoutMoveIntent } from './loadout-panel';

@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LoadoutPanelComponent],
  selector: 'rr-preparation',
  standalone: true,
  styles: [
    `
      :host {
        inset: 0;
        pointer-events: none;
        position: absolute;
        z-index: 3;
      }

      .preparation {
        backdrop-filter: blur(10px);
        background: rgb(4 13 17 / 0.78);
        border: 1px solid var(--rr-line);
        border-radius: 12px;
        bottom: 1rem;
        display: grid;
        gap: 0.8rem;
        grid-template-rows: auto auto minmax(0, 1fr) auto;
        left: 50%;
        max-width: 1180px;
        overflow: hidden;
        padding: 0.85rem;
        pointer-events: auto;
        position: absolute;
        top: 1rem;
        transform: translateX(-50%);
        width: calc(100vw - 2rem);
      }

      header,
      .tabs,
      footer {
        align-items: center;
        display: flex;
        gap: 0.5rem;
      }

      header,
      footer {
        justify-content: space-between;
      }

      h1,
      p {
        margin: 0;
      }

      .eyebrow,
      .instructions {
        color: var(--rr-muted);
      }

      .eyebrow {
        font-size: 0.7rem;
        font-weight: 800;
        letter-spacing: 0.12em;
        text-transform: uppercase;
      }

      .tabs {
        overflow-x: auto;
      }

      button {
        background: rgb(8 20 24 / 0.9);
        border: 1px solid var(--rr-line);
        border-radius: 7px;
        color: var(--rr-text);
        cursor: pointer;
        flex: 0 0 auto;
        font: inherit;
        min-height: 44px;
        padding: 0.55rem 0.75rem;
      }

      button:hover:not(:disabled),
      button:focus-visible,
      button[aria-selected='true'] {
        background: rgb(126 229 210 / 0.15);
        border-color: var(--rr-accent);
        outline: none;
      }

      button:disabled {
        cursor: not-allowed;
        opacity: 0.45;
      }

      .workspace {
        display: grid;
        gap: 0.8rem;
        grid-template-columns: minmax(0, 1fr) minmax(280px, 0.82fr);
        min-height: 0;
        overflow: auto;
      }

      .loadout,
      .stash {
        background: rgb(9 24 29 / 0.76);
        border: 1px solid var(--rr-line);
        border-radius: 9px;
        min-width: 0;
        overflow: auto;
        padding: 0.75rem;
      }

      .selection {
        color: var(--rr-accent);
        min-height: 1.2em;
      }

      .error {
        color: #ffb4a9;
      }

      @media (max-width: 720px) {
        .preparation {
          bottom: 0.35rem;
          padding: 0.65rem;
          top: 0.35rem;
          width: calc(100vw - 0.7rem);
        }

        header {
          align-items: flex-start;
          flex-direction: column;
        }

        .workspace {
          grid-template-columns: 1fr;
        }

        footer {
          align-items: stretch;
          flex-direction: column;
        }
      }
    `,
  ],
  template: `
    @if (view().preparation; as preparation) {
      <section class="preparation" aria-labelledby="preparation-title">
        <header>
          <div>
            <p class="eyebrow">Lantern Company</p>
            <h1 id="preparation-title">Prepare the expedition</h1>
          </div>
          <p class="instructions">
            Drag gear, or select an item then choose its matching equipment
            slot.
          </p>
        </header>

        <nav class="tabs" aria-label="Prepare party member">
          @for (member of view().party; track member.entityId) {
            <button
              type="button"
              [attr.aria-pressed]="activeMember()?.entityId === member.entityId"
              (click)="activeMemberId.set(member.entityId)"
            >
              {{ member.name }} · {{ member.className }}
            </button>
          }
        </nav>

        <div class="workspace">
          @if (activeMember(); as member) {
            <section
              class="loadout"
              [attr.aria-label]="member.name + ' loadout'"
            >
              <rr-loadout-panel
                [loadout]="member.loadout"
                [readOnly]="store.busy()"
                [selected]="selected()"
                [title]="member.name + ' · ' + member.title"
                (itemSelected)="select($event)"
                (moveRequested)="move($event)"
              />
            </section>
          }

          <section class="stash" aria-label="Shared stash">
            <rr-loadout-panel
              [loadout]="preparation.stash"
              [readOnly]="store.busy()"
              [selected]="selected()"
              title="Shared stash"
              (itemSelected)="select($event)"
              (moveRequested)="move($event)"
            />
          </section>
        </div>

        <footer>
          <div>
            <p class="selection" aria-live="polite">{{ selectionLabel() }}</p>
            @if (store.commandError(); as failure) {
              <p class="error" role="alert">
                @if (failure.code !== null) {
                  <strong>{{ failure.code }}</strong> ·
                }
                {{ failure.detail }}
              </p>
            }
          </div>
          <button
            type="button"
            [disabled]="!preparation.ready || store.busy()"
            (click)="begin()"
          >
            {{ store.busy() ? 'Applying…' : 'Begin expedition' }}
          </button>
        </footer>
      </section>
    }
  `,
})
export class PreparationComponent {
  readonly view = input.required<SessionView>();
  protected readonly store = inject(SessionStore);
  protected readonly activeMemberId = signal<number | null>(null);
  protected readonly selected = signal<LoadoutDragPayload | null>(null);
  protected readonly activeMember = computed(
    () =>
      this.view().party.find(
        (member) => member.entityId === this.activeMemberId(),
      ) ??
      this.view().party[0] ??
      null,
  );
  protected readonly selectionLabel = computed(() => {
    const selected = this.selected();
    if (selected === null) {
      return 'No item selected.';
    }
    for (const loadout of [
      ...this.view().party.map((member) => member.loadout),
      this.view().preparation?.stash,
    ]) {
      const item = loadout?.inventorySlots.find(
        (candidate) => candidate?.entityId === selected.itemEntityId,
      );
      if (item !== undefined && item !== null) {
        return `${item.name} selected. Choose a destination.`;
      }
    }
    return 'Selected item is no longer available.';
  });

  constructor() {
    effect(() => {
      const party = this.view().party;
      const first = party[0];
      if (
        first !== undefined &&
        !party.some((member) => member.entityId === this.activeMemberId())
      ) {
        this.activeMemberId.set(first.entityId);
      }
    });
  }

  protected select(payload: LoadoutDragPayload): void {
    this.selected.set(
      this.selected()?.itemEntityId === payload.itemEntityId ? null : payload,
    );
  }

  protected async move(intent: LoadoutMoveIntent): Promise<void> {
    if (this.store.busy()) {
      return;
    }
    const accepted = await this.store.command({
      kind: 'moveLoadoutItem',
      expectedRevision: this.view().revision,
      itemEntityId: intent.itemEntityId,
      fromOwnerEntityId: intent.ownerEntityId,
      toOwnerEntityId: intent.toOwnerEntityId,
      destinationSlotId: intent.destinationSlotId,
    });
    if (accepted) {
      this.selected.set(null);
    }
  }

  protected async begin(): Promise<void> {
    if (!this.view().preparation?.ready || this.store.busy()) {
      return;
    }
    await this.store.command({
      kind: 'beginExpedition',
      expectedRevision: this.view().revision,
    });
  }
}
