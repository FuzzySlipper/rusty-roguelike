import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  input,
  signal,
  viewChildren,
  type ElementRef,
} from '@angular/core';

import type { PartyMemberStatusView } from '@rusty-roguelike/protocol';

import { LoadoutPanelComponent } from './loadout-panel';

@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [LoadoutPanelComponent],
  selector: 'rr-party-sheet',
  standalone: true,
  styles: [
    `
      :host {
        display: grid;
        gap: 0.8rem;
        min-width: 0;
      }

      .tabs {
        display: flex;
        gap: 0.35rem;
        overflow-x: auto;
      }

      button {
        background: rgb(8 20 24 / 0.84);
        border: 1px solid var(--rr-line);
        border-radius: 7px;
        color: var(--rr-text);
        cursor: pointer;
        flex: 0 0 auto;
        font: inherit;
        min-height: 44px;
        padding: 0.55rem 0.75rem;
      }

      button[aria-selected='true'],
      button:focus-visible {
        background: rgb(126 229 210 / 0.15);
        border-color: var(--rr-accent);
        outline: none;
      }

      .summary,
      .facts,
      .cards {
        display: grid;
        gap: 0.55rem;
      }

      .summary {
        grid-template-columns: minmax(0, 1fr) auto;
      }

      h3,
      h4,
      p,
      ul {
        margin: 0;
      }

      .eyebrow,
      small {
        color: var(--rr-muted);
      }

      .facts {
        grid-template-columns: repeat(auto-fit, minmax(112px, 1fr));
      }

      .fact,
      article {
        background: rgb(8 20 24 / 0.62);
        border: 1px solid var(--rr-line);
        border-radius: 7px;
        padding: 0.55rem;
      }

      .fact {
        display: grid;
        gap: 0.15rem;
      }

      .cards {
        grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
      }

      section {
        display: grid;
        gap: 0.45rem;
      }
    `,
  ],
  template: `
    <div class="tabs" role="tablist" aria-label="Party members">
      @for (member of party(); track member.entityId; let index = $index) {
        <button
          #partyTab
          type="button"
          role="tab"
          [id]="'party-tab-' + member.entityId"
          [attr.aria-controls]="'party-panel-' + member.entityId"
          [attr.aria-selected]="active()?.entityId === member.entityId"
          [attr.tabindex]="active()?.entityId === member.entityId ? 0 : -1"
          (click)="select(member.entityId)"
          (keydown)="tabKeydown($event, index)"
        >
          {{ member.name }}
        </button>
      }
    </div>

    @if (active(); as member) {
      <div
        role="tabpanel"
        [id]="'party-panel-' + member.entityId"
        [attr.aria-labelledby]="'party-tab-' + member.entityId"
      >
        <header class="summary">
          <div>
            <p class="eyebrow">{{ member.title }}</p>
            <h3>{{ member.name }}</h3>
            <p>
              Level {{ member.level }} {{ member.className }} ·
              {{ member.experience }} XP
            </p>
          </div>
          <strong>
            {{ member.currentVitality }} / {{ member.maximumVitality }} vitality
          </strong>
        </header>

        <section aria-label="Abilities">
          <h4>Abilities</h4>
          <div class="facts">
            @for (ability of member.abilities; track ability.abilityId) {
              <span class="fact">
                <small>{{ ability.abilityId }}</small>
                <strong>
                  {{ ability.score }} ({{ signed(ability.modifier) }})
                </strong>
              </span>
            }
          </div>
        </section>

        <section aria-label="Defenses">
          <h4>Defenses</h4>
          <div class="facts">
            @for (defense of member.defenses; track defense.defenseId) {
              <span class="fact">
                <small>{{ defense.defenseId }}</small>
                <strong>{{ defense.value }}</strong>
              </span>
            }
          </div>
        </section>

        <section aria-label="Features and feats">
          <h4>Features &amp; feats</h4>
          <div class="cards">
            @for (feat of member.feats; track feat.featId) {
              <article>
                <strong>{{ feat.name }}</strong>
                <p>{{ feat.description }}</p>
              </article>
            }
          </div>
        </section>

        <section aria-label="Actions">
          <h4>Actions</h4>
          <ul>
            @for (action of member.actions; track action.actionId) {
              <li>{{ action.name }}</li>
            }
          </ul>
        </section>

        @if (showLoadout()) {
          <rr-loadout-panel
            [loadout]="member.loadout"
            [readOnly]="true"
            [title]="member.name + ' pack'"
          />
        }
      </div>
    }
  `,
})
export class PartySheetComponent {
  readonly party = input.required<readonly PartyMemberStatusView[]>();
  readonly showLoadout = input(false);
  protected readonly activeId = signal<number | null>(null);
  protected readonly active = computed(
    () =>
      this.party().find((member) => member.entityId === this.activeId()) ??
      this.party()[0] ??
      null,
  );
  private readonly tabs =
    viewChildren<ElementRef<HTMLButtonElement>>('partyTab');

  constructor() {
    effect(() => {
      const party = this.party();
      const first = party[0];
      if (
        first !== undefined &&
        !party.some((member) => member.entityId === this.activeId())
      ) {
        this.activeId.set(first.entityId);
      }
    });
  }

  protected select(entityId: number): void {
    this.activeId.set(entityId);
  }

  protected tabKeydown(event: KeyboardEvent, index: number): void {
    const maximum = this.party().length - 1;
    const next =
      event.key === 'Home'
        ? 0
        : event.key === 'End'
          ? maximum
          : event.key === 'ArrowRight'
            ? (index + 1) % (maximum + 1)
            : event.key === 'ArrowLeft'
              ? (index + maximum) % (maximum + 1)
              : null;
    if (next === null) {
      return;
    }
    event.preventDefault();
    const member = this.party()[next];
    if (member !== undefined) {
      this.activeId.set(member.entityId);
      this.tabs()[next]?.nativeElement.focus();
    }
  }

  protected signed(value: number): string {
    return value >= 0 ? `+${value}` : String(value);
  }
}
