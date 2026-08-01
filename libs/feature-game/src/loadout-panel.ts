import {
  ChangeDetectionStrategy,
  Component,
  input,
  output,
} from '@angular/core';

import {
  admitLoadoutDrag,
  markLoadoutDragMove,
  readLoadoutDrag,
  writeLoadoutDrag,
  type LoadoutDragPayload,
} from '@rusty-roguelike/platform';
import type {
  EquipmentSlotView,
  LoadoutItemView,
  LoadoutView,
} from '@rusty-roguelike/protocol';

export interface LoadoutMoveIntent extends LoadoutDragPayload {
  readonly toOwnerEntityId: number;
  readonly destinationSlotId: string | null;
}

@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  selector: 'rr-loadout-panel',
  standalone: true,
  styles: [
    `
      :host {
        display: grid;
        gap: 0.8rem;
        min-width: 0;
      }

      header,
      .capacity {
        align-items: center;
        display: flex;
        gap: 0.55rem;
        justify-content: space-between;
      }

      h3,
      p {
        margin: 0;
      }

      h3 {
        font-size: 1rem;
      }

      .capacity {
        color: var(--rr-muted);
        font-size: 0.75rem;
      }

      .slots {
        display: grid;
        gap: 0.45rem;
        grid-template-columns: repeat(auto-fit, minmax(112px, 1fr));
      }

      button {
        background: rgb(8 20 24 / 0.84);
        border: 1px solid var(--rr-line);
        border-radius: 7px;
        color: var(--rr-text);
        cursor: pointer;
        font: inherit;
        min-height: 48px;
        min-width: 0;
        padding: 0.55rem;
        text-align: left;
      }

      button:hover:not(:disabled),
      button:focus-visible,
      button.selected,
      button.drop-target {
        background: rgb(126 229 210 / 0.15);
        border-color: var(--rr-accent);
        outline: none;
      }

      button:disabled {
        cursor: default;
        opacity: 0.72;
      }

      .equipment button {
        display: grid;
        gap: 0.2rem;
      }

      .slot-label,
      .item-meta {
        color: var(--rr-muted);
        font-size: 0.68rem;
        letter-spacing: 0.08em;
        text-transform: uppercase;
      }

      .empty {
        color: var(--rr-muted);
        font-style: italic;
      }

      .pack-destination {
        border-style: dashed;
        text-align: center;
      }
    `,
  ],
  template: `
    <header>
      <div>
        <p class="slot-label">Loadout</p>
        <h3>{{ title() }}</h3>
      </div>
      <span class="capacity">
        {{ loadout().capacity.used }} / {{ loadout().capacity.maximum }}
      </span>
    </header>

    @if (loadout().equipmentSlots.length > 0) {
      <section class="equipment" aria-label="Equipment slots">
        <p class="slot-label">Equipment</p>
        <div class="slots">
          @for (slot of loadout().equipmentSlots; track slot.slotId) {
            <button
              type="button"
              [attr.aria-label]="slotLabel(slot)"
              [class.drop-target]="selected() !== null && !readOnly()"
              [disabled]="readOnly()"
              (click)="moveSelected(slot.slotId)"
              (dragover)="dragOver($event)"
              (drop)="drop($event, slot.slotId)"
            >
              <span class="slot-label">{{ slot.label }}</span>
              @if (slot.equipped; as equipped) {
                <strong>{{ equipped.name }}</strong>
              } @else {
                <span class="empty">Empty</span>
              }
            </button>
          }
        </div>
      </section>
    }

    <section aria-label="Inventory slots">
      <div class="capacity">
        <span class="slot-label">Pack</span>
        @if (!readOnly() && selected() !== null) {
          <button
            class="pack-destination"
            type="button"
            (click)="moveSelected(null)"
            (dragover)="dragOver($event)"
            (drop)="drop($event, null)"
          >
            Move selected to pack
          </button>
        }
      </div>
      <div class="slots">
        @for (item of loadout().inventorySlots; track $index) {
          @if (item !== null) {
            <button
              type="button"
              [attr.draggable]="readOnly() ? null : true"
              [disabled]="readOnly()"
              [class.selected]="selected()?.itemEntityId === item.entityId"
              [attr.aria-pressed]="selected()?.itemEntityId === item.entityId"
              (click)="select(item)"
              (dragstart)="dragStart($event, item)"
            >
              <strong>{{ item.name }}</strong>
              <span class="item-meta">
                {{ item.equippedSlotId ?? item.equipmentSlotId ?? 'carried' }}
              </span>
            </button>
          } @else {
            <button type="button" disabled>
              <span class="empty">Empty</span>
            </button>
          }
        }
      </div>
    </section>
  `,
})
export class LoadoutPanelComponent {
  readonly loadout = input.required<LoadoutView>();
  readonly readOnly = input(false);
  readonly selected = input<LoadoutDragPayload | null>(null);
  readonly title = input.required<string>();
  readonly itemSelected = output<LoadoutDragPayload>();
  readonly moveRequested = output<LoadoutMoveIntent>();

  protected select(item: LoadoutItemView): void {
    if (!this.readOnly()) {
      this.itemSelected.emit({
        itemEntityId: item.entityId,
        ownerEntityId: this.loadout().ownerEntityId,
      });
    }
  }

  protected moveSelected(destinationSlotId: string | null): void {
    const selected = this.selected();
    if (this.readOnly() || selected === null) {
      return;
    }
    this.moveRequested.emit({
      ...selected,
      toOwnerEntityId: this.loadout().ownerEntityId,
      destinationSlotId,
    });
  }

  protected dragStart(event: DragEvent, item: LoadoutItemView): void {
    if (this.readOnly()) {
      event.preventDefault();
      return;
    }
    writeLoadoutDrag(event.dataTransfer, {
      itemEntityId: item.entityId,
      ownerEntityId: this.loadout().ownerEntityId,
    });
  }

  protected dragOver(event: DragEvent): void {
    if (!this.readOnly() && admitLoadoutDrag(event.dataTransfer)) {
      event.preventDefault();
      markLoadoutDragMove(event.dataTransfer);
    }
  }

  protected drop(event: DragEvent, destinationSlotId: string | null): void {
    if (this.readOnly()) {
      return;
    }
    const payload = readLoadoutDrag(event.dataTransfer);
    if (payload === null) {
      return;
    }
    event.preventDefault();
    this.moveRequested.emit({
      ...payload,
      toOwnerEntityId: this.loadout().ownerEntityId,
      destinationSlotId,
    });
  }

  protected slotLabel(slot: EquipmentSlotView): string {
    return `${slot.label}: ${slot.equipped?.name ?? 'empty'}`;
  }
}
