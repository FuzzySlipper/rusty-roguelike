import {
  afterRenderEffect,
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  inject,
  signal,
  viewChild,
  type ElementRef,
  type OnDestroy,
  type OnInit,
} from '@angular/core';

import {
  keyboardEventTargetsInteractive,
  keyboardEventTargetsEditable,
  observeGlobalKeydown,
} from '@rusty-roguelike/platform';
import type {
  LegalActionView,
  RelativeStep,
  SessionCommandDto,
  TurnReceipt,
} from '@rusty-roguelike/protocol';
import { GameViewportComponent } from '@rusty-roguelike/renderer';
import {
  BootstrapStore,
  SessionStore,
  type RulesLogEntry,
} from '@rusty-roguelike/store';

import { PartySheetComponent } from './party-sheet';
import { MinimapComponent } from './minimap';
import { PreparationComponent } from './preparation';

type Drawer = 'party' | 'inventory' | null;

@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    GameViewportComponent,
    MinimapComponent,
    PartySheetComponent,
    PreparationComponent,
  ],
  selector: 'rr-game-shell',
  standalone: true,
  styles: [
    `
      :host,
      main,
      .stage {
        display: block;
        height: 100dvh;
        inset: 0;
        overflow: hidden;
        position: fixed;
        width: 100vw;
      }

      .loading,
      .fatal {
        align-items: center;
        background:
          radial-gradient(
            circle at 50% 30%,
            rgb(31 67 66 / 0.65),
            transparent 42%
          ),
          var(--rr-bg);
        display: grid;
        justify-items: center;
        padding: 2rem;
        text-align: center;
      }

      .loading h1,
      .fatal h1 {
        font-size: clamp(2.4rem, 8vw, 5rem);
        margin: 0 0 0.75rem;
      }

      .hud {
        inset: 0;
        pointer-events: none;
        position: absolute;
        z-index: 2;
      }

      .panel,
      button,
      details {
        pointer-events: auto;
      }

      .panel {
        backdrop-filter: blur(12px);
        background: var(--rr-panel);
        border: 1px solid var(--rr-line);
        border-radius: 9px;
        box-shadow: 0 12px 36px rgb(0 0 0 / 0.28);
      }

      .initiative {
        display: flex;
        gap: 0.35rem;
        left: 50%;
        max-width: min(70vw, 760px);
        overflow-x: auto;
        padding: 0.38rem;
        position: absolute;
        top: 0.65rem;
        transform: translateX(-50%);
      }

      .initiative span {
        border: 1px solid transparent;
        border-radius: 999px;
        color: var(--rr-muted);
        flex: 0 0 auto;
        font-size: 0.72rem;
        padding: 0.35rem 0.55rem;
      }

      .initiative .current {
        background: rgb(126 229 210 / 0.14);
        border-color: var(--rr-accent);
        color: var(--rr-text);
      }

      .objective {
        left: 50%;
        padding: 0.5rem 0.7rem;
        position: absolute;
        text-align: center;
        top: 3.75rem;
        transform: translateX(-50%);
      }

      .objective strong,
      .objective span {
        display: block;
      }

      .objective strong {
        font-size: 0.82rem;
      }

      .objective span {
        color: var(--rr-muted);
        font-size: 0.7rem;
        margin-top: 0.15rem;
      }

      .objective.complete {
        border-color: var(--rr-accent);
        box-shadow: 0 0 30px rgb(126 229 210 / 0.18);
      }

      .map-cluster {
        display: grid;
        gap: 0.35rem;
        max-width: calc(100vw - 1.3rem);
        pointer-events: none;
        position: absolute;
        right: 0.65rem;
        top: 0.65rem;
        width: min(260px, 22vw);
      }

      .map-toolbar {
        display: flex;
        gap: 0.25rem;
        padding: 0.28rem;
        pointer-events: auto;
      }

      .map-toolbar button {
        flex: 1 1 auto;
        font-size: 0.72rem;
        min-width: 0;
        padding: 0.4rem 0.32rem;
      }

      .game-menu-layer {
        display: grid;
        inset: 0;
        place-items: center;
        pointer-events: auto;
        position: absolute;
        z-index: 8;
      }

      .game-menu-backdrop {
        background: rgb(3 9 11 / 0.72);
        inset: 0;
        position: absolute;
      }

      .game-menu {
        max-width: min(420px, calc(100vw - 2rem));
        padding: 1rem;
        position: relative;
        width: 100%;
        z-index: 1;
      }

      .game-menu header {
        align-items: center;
        display: flex;
        justify-content: space-between;
      }

      .game-menu h2 {
        margin: 0;
      }

      .game-menu-actions {
        display: grid;
        gap: 0.5rem;
        margin-top: 1rem;
      }

      .game-menu-actions button {
        text-align: left;
      }

      .game-menu-note {
        color: var(--rr-muted);
        font-size: 0.78rem;
        margin: 0.8rem 0 0;
      }

      .game-menu-error {
        background: rgb(72 17 20 / 0.96);
        border: 1px solid rgb(255 135 135 / 0.55);
        color: #ffd9d1;
        margin: 0.8rem 0 0;
        padding: 0.7rem 0.9rem;
      }

      .map-persistence-notice {
        background: rgb(7 17 20 / 0.94);
        border: 1px solid var(--rr-accent);
        border-radius: 5px;
        color: var(--rr-accent);
        font-size: 0.68rem;
        justify-self: end;
        padding: 0.24rem 0.4rem;
        pointer-events: none;
      }

      .persistence-tools {
        display: flex;
        gap: 0.4rem;
        pointer-events: auto;
        position: absolute;
        right: 0.65rem;
        top: 0.65rem;
        z-index: 4;
      }

      .persistence-notice {
        align-self: center;
        color: var(--rr-accent);
        font-size: 0.72rem;
      }

      button {
        background: rgb(12 27 31 / 0.96);
        border: 1px solid var(--rr-line);
        border-radius: 7px;
        color: var(--rr-text);
        cursor: pointer;
        font: inherit;
        min-height: 44px;
        padding: 0.55rem 0.75rem;
      }

      button:hover:not(:disabled),
      button:focus-visible,
      button.selected {
        background: rgb(126 229 210 / 0.16);
        border-color: var(--rr-accent);
        outline: none;
      }

      button:disabled {
        cursor: not-allowed;
        opacity: 0.38;
      }

      .party-rail {
        display: grid;
        gap: 0.45rem;
        left: 0.65rem;
        max-width: 190px;
        padding: 0.55rem;
        position: absolute;
        top: 7rem;
        width: calc(100vw - 1.3rem);
      }

      .member {
        display: grid;
        gap: 0.25rem;
      }

      .member header {
        display: flex;
        font-size: 0.78rem;
        justify-content: space-between;
      }

      .member meter {
        accent-color: var(--rr-accent);
        height: 0.42rem;
        width: 100%;
      }

      .member.down {
        opacity: 0.58;
      }

      .actions {
        bottom: 0.65rem;
        left: 0.65rem;
        max-height: 35dvh;
        max-width: min(460px, calc(100vw - 1.3rem));
        overflow: auto;
        padding: 0.65rem;
        position: absolute;
        width: max-content;
      }

      .panel-title {
        color: var(--rr-accent);
        font-size: 0.68rem;
        font-weight: 800;
        letter-spacing: 0.11em;
        margin: 0 0 0.45rem;
        text-transform: uppercase;
      }

      .action-row,
      .target-row {
        display: flex;
        flex-wrap: wrap;
        gap: 0.4rem;
      }

      .target-row {
        border-top: 1px solid var(--rr-line);
        margin-top: 0.55rem;
        padding-top: 0.55rem;
      }

      .movement {
        bottom: 0.65rem;
        display: grid;
        gap: 0.3rem;
        grid-template-columns: repeat(3, 46px);
        left: 50%;
        padding: 0.45rem;
        position: absolute;
        transform: translateX(-50%);
      }

      .movement button {
        min-height: 46px;
        padding: 0;
      }

      .movement .forward {
        grid-column: 2;
      }

      .movement .turn-left {
        grid-column: 1;
      }

      .movement .backward {
        grid-column: 2;
      }

      .rules-log {
        bottom: 0.65rem;
        max-height: 34dvh;
        max-width: min(390px, calc(100vw - 1.3rem));
        overflow: auto;
        padding: 0.6rem;
        position: absolute;
        right: 0.65rem;
        width: 30vw;
      }

      .empty-log {
        color: var(--rr-muted);
        font-size: 0.78rem;
        margin: 0;
      }

      .log-entry {
        border-top: 1px solid var(--rr-line);
        font-size: 0.76rem;
        padding: 0.42rem 0;
      }

      .log-entry:first-of-type {
        border-top: 0;
      }

      .log-entry summary {
        cursor: help;
      }

      .log-detail {
        color: var(--rr-muted);
        font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
        line-height: 1.45;
        margin: 0.35rem 0 0;
        overflow-wrap: anywhere;
      }

      .drawer {
        left: 50%;
        max-height: min(70dvh, 620px);
        max-width: min(620px, calc(100vw - 2rem));
        overflow: auto;
        padding: 1rem;
        position: absolute;
        top: 50%;
        transform: translate(-50%, -50%);
        width: 100%;
      }

      .drawer header {
        align-items: center;
        display: flex;
        justify-content: space-between;
      }

      .drawer h2 {
        margin: 0;
      }

      .drawer-grid {
        display: grid;
        gap: 0.7rem;
        grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
        margin-top: 1rem;
      }

      .drawer article {
        background: rgb(255 255 255 / 0.035);
        border: 1px solid var(--rr-line);
        padding: 0.75rem;
      }

      .drawer h3 {
        margin: 0 0 0.4rem;
      }

      .drawer ul {
        color: var(--rr-muted);
        margin: 0;
        padding-left: 1.1rem;
      }

      .command-error {
        background: rgb(72 17 20 / 0.96);
        border: 1px solid rgb(255 135 135 / 0.55);
        color: #ffd9d1;
        left: 50%;
        max-width: min(560px, calc(100vw - 2rem));
        padding: 0.7rem 0.9rem;
        position: absolute;
        top: 4.2rem;
        transform: translateX(-50%);
      }

      .system-readout {
        bottom: 0.3rem;
        color: transparent;
        font-size: 1px;
        pointer-events: none;
        position: absolute;
        right: 0.3rem;
      }

      @media (max-width: 760px) {
        .initiative {
          left: 0.5rem;
          max-width: calc(52vw - 0.75rem);
          transform: none;
        }

        .objective {
          left: 0.5rem;
          max-width: calc(52vw - 0.75rem);
          text-align: left;
          transform: none;
        }

        .map-cluster {
          right: 0.5rem;
          top: 0.5rem;
          width: calc(48vw - 0.75rem);
        }

        .map-toolbar {
          gap: 0.15rem;
          padding: 0.2rem;
        }

        .map-toolbar button {
          font-size: 0.66rem;
          padding: 0.35rem 0.18rem;
        }

        .persistence-notice {
          position: absolute;
          right: 0;
          top: calc(100% + 0.2rem);
          white-space: nowrap;
        }

        .party-rail {
          display: grid;
          left: 0.5rem;
          max-width: calc(52vw - 0.75rem);
          top: 9rem;
        }

        .member {
          min-width: 0;
        }

        .actions {
          bottom: 0.5rem;
          left: 0.5rem;
          max-height: 11rem;
          max-width: calc(30vw - 0.5rem);
          width: 100%;
        }

        .action-row,
        .target-row {
          display: grid;
        }

        .actions button {
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }

        .movement {
          bottom: 0.5rem;
        }

        .rules-log {
          bottom: 0.5rem;
          max-height: 11rem;
          max-width: calc(30vw - 0.5rem);
          right: 0.5rem;
          width: 100%;
        }
      }
    `,
  ],
  template: `
    <main>
      @switch (session.state().status) {
        @case ('loading') {
          <section class="stage loading" role="status">
            <div>
              <p class="panel-title">Rust-owned expedition</p>
              <h1>Rusty Roguelike</h1>
              <p>Generating the floor and assembling initiative…</p>
            </div>
          </section>
        }
        @case ('error') {
          @if (session.state(); as state) {
            @if (state.status === 'error') {
              <section class="stage fatal" role="alert">
                <div>
                  <p class="panel-title">Expedition unavailable</p>
                  <h1>Rusty Roguelike</h1>
                  <p>{{ state.message }}</p>
                  <button type="button" (click)="reload()">Retry</button>
                </div>
              </section>
            }
          }
        }
        @case ('ready') {
          @if (session.state(); as state) {
            @if (state.status === 'ready') {
              <section
                class="stage"
                [attr.data-session-revision]="state.value.revision"
                [attr.data-session-outcome]="state.value.outcome"
                [attr.data-visible-enemies]="
                  state.value.world.visibleActors.length
                "
              >
                <rr-game-viewport
                  [session]="state.value"
                  [selectedActionId]="selectedActionId()"
                  (actorPicked)="pickTarget($event)"
                />
                @if (state.value.phase === 'preparation') {
                  <div class="persistence-tools">
                    @if (session.persistenceNotice(); as notice) {
                      <span class="persistence-notice" role="status">{{
                        notice
                      }}</span>
                    }
                    <button
                      #gameMenuTrigger
                      type="button"
                      aria-haspopup="dialog"
                      [attr.aria-expanded]="gameMenuOpen()"
                      (click)="openGameMenu()"
                    >
                      Menu
                    </button>
                  </div>
                  <rr-preparation [view]="state.value" />
                } @else {
                  <div class="hud">
                    <nav class="panel initiative" aria-label="Initiative order">
                      @for (
                        activation of state.value.order;
                        track activation.entityId
                      ) {
                        <span
                          [class.current]="
                            activation.entityId ===
                            state.value.current?.entityId
                          "
                          [attr.aria-current]="
                            activation.entityId ===
                            state.value.current?.entityId
                              ? 'step'
                              : null
                          "
                        >
                          {{ activation.name }} · {{ activation.initiative }}
                        </span>
                      }
                    </nav>

                    <section
                      class="panel objective"
                      [class.complete]="state.value.outcome === 'victory'"
                      aria-label="Floor objective"
                      role="status"
                    >
                      @switch (state.value.outcome) {
                        @case ('victory') {
                          <strong>Ember den secured</strong>
                          <span
                            >Every hostile is down. The floor is complete.</span
                          >
                        }
                        @case ('defeat') {
                          <strong>Expedition lost</strong>
                          <span>The party can no longer continue.</span>
                        }
                        @default {
                          <strong>Purge the ember den</strong>
                          <span
                            >Find and defeat all fifteen dormant raiders.</span
                          >
                        }
                      }
                    </section>

                    <aside
                      class="map-cluster"
                      aria-label="Map and expedition menu"
                    >
                      <div class="panel map-toolbar">
                        <button
                          #partyTrigger
                          type="button"
                          aria-controls="party-drawer"
                          [attr.aria-expanded]="drawer() === 'party'"
                          (click)="openDrawer('party')"
                        >
                          Party
                        </button>
                        <button
                          #inventoryTrigger
                          type="button"
                          aria-controls="inventory-drawer"
                          [attr.aria-expanded]="drawer() === 'inventory'"
                          (click)="openDrawer('inventory')"
                        >
                          Packs
                        </button>
                        <button
                          #gameMenuTrigger
                          type="button"
                          aria-haspopup="dialog"
                          [attr.aria-expanded]="gameMenuOpen()"
                          (click)="openGameMenu()"
                        >
                          Menu
                        </button>
                      </div>
                      @if (session.persistenceNotice(); as notice) {
                        <span class="map-persistence-notice" role="status">{{
                          notice
                        }}</span>
                      }
                      <rr-minimap
                        [minimap]="state.value.world.minimap"
                        [revision]="state.value.revision"
                      />
                    </aside>

                    <aside class="panel party-rail" aria-label="Party vitality">
                      @for (
                        member of state.value.party;
                        track member.entityId
                      ) {
                        <div class="member" [class.down]="!member.conscious">
                          <header>
                            <span>{{ member.name }}</span>
                            <span
                              >{{ member.currentVitality }}/{{
                                member.maximumVitality
                              }}</span
                            >
                          </header>
                          <meter
                            min="0"
                            [max]="member.maximumVitality"
                            [value]="member.currentVitality"
                          >
                            {{ member.currentVitality }} of
                            {{ member.maximumVitality }}
                          </meter>
                        </div>
                      }
                    </aside>

                    <section
                      class="panel actions"
                      aria-label="Available actions"
                    >
                      <p class="panel-title">
                        One action · {{ state.value.current?.name }}
                      </p>
                      <div class="action-row">
                        <button
                          type="button"
                          aria-label="Wait (Space)"
                          [disabled]="
                            !state.value.decision?.canWait || session.busy()
                          "
                          (click)="wait()"
                        >
                          Space · Wait
                        </button>
                        @for (
                          action of state.value.decision?.actions ?? [];
                          track action.actionId;
                          let index = $index
                        ) {
                          <button
                            type="button"
                            [class.selected]="
                              selectedActionId() === action.actionId
                            "
                            [disabled]="
                              session.busy() ||
                              action.legalTargetEntityIds.length === 0
                            "
                            [attr.aria-pressed]="
                              selectedActionId() === action.actionId
                            "
                            (click)="selectAction(action)"
                          >
                            {{ index + 1 }} · {{ action.name }}
                          </button>
                        }
                      </div>
                      @if (selectedAction(); as action) {
                        <div class="target-row" aria-label="Legal targets">
                          @for (
                            targetId of action.legalTargetEntityIds;
                            track targetId
                          ) {
                            <button
                              type="button"
                              [disabled]="session.busy()"
                              (click)="useAction(action, targetId)"
                            >
                              {{ targetName(targetId) }}
                            </button>
                          }
                        </div>
                      }
                    </section>

                    <nav
                      class="panel movement"
                      aria-label="Movement and facing"
                    >
                      <button
                        class="forward"
                        type="button"
                        aria-label="Step forward"
                        [disabled]="!canStep('forward') || session.busy()"
                        (click)="step('forward')"
                      >
                        W
                      </button>
                      <button
                        class="turn-left"
                        type="button"
                        aria-label="Turn left"
                        [disabled]="
                          !state.value.decision?.canTurn || session.busy()
                        "
                        (click)="turn('left')"
                      >
                        Q
                      </button>
                      <button
                        type="button"
                        aria-label="Step left"
                        [disabled]="!canStep('left') || session.busy()"
                        (click)="step('left')"
                      >
                        A
                      </button>
                      <button
                        type="button"
                        aria-label="Step right"
                        [disabled]="!canStep('right') || session.busy()"
                        (click)="step('right')"
                      >
                        D
                      </button>
                      <button
                        type="button"
                        aria-label="Turn right"
                        [disabled]="
                          !state.value.decision?.canTurn || session.busy()
                        "
                        (click)="turn('right')"
                      >
                        E
                      </button>
                      <button
                        class="backward"
                        type="button"
                        aria-label="Step backward"
                        [disabled]="!canStep('backward') || session.busy()"
                        (click)="step('backward')"
                      >
                        S
                      </button>
                    </nav>

                    <section
                      #rulesLog
                      class="panel rules-log"
                      aria-label="Rules log"
                      aria-live="polite"
                    >
                      <p class="panel-title">Rules log</p>
                      @if (session.log().length === 0) {
                        <p class="empty-log">
                          The expedition is waiting for a command.
                        </p>
                      }
                      @for (entry of session.log(); track entry.id) {
                        <details
                          class="log-entry"
                          [title]="receiptDetail(entry)"
                        >
                          <summary>{{ receiptSummary(entry) }}</summary>
                          <p class="log-detail">{{ receiptDetail(entry) }}</p>
                        </details>
                      }
                    </section>

                    @if (drawer(); as open) {
                      <section
                        [attr.id]="
                          open === 'party' ? 'party-drawer' : 'inventory-drawer'
                        "
                        class="panel drawer"
                        role="region"
                        [attr.aria-label]="
                          open === 'party' ? 'Party quick view' : 'Field packs'
                        "
                      >
                        <header>
                          <h2>
                            {{
                              open === 'party'
                                ? 'Party quick view'
                                : 'Field packs'
                            }}
                          </h2>
                          <button
                            type="button"
                            aria-label="Close panel"
                            (click)="closeDrawer()"
                          >
                            ×
                          </button>
                        </header>
                        <rr-party-sheet
                          [party]="state.value.party"
                          [showLoadout]="open === 'inventory'"
                        />
                      </section>
                    }

                    @if (session.commandError(); as failure) {
                      <p class="command-error" role="alert">
                        @if (failure.code !== null) {
                          <strong>{{ failure.code }}</strong> ·
                        }
                        {{ failure.detail }}
                      </p>
                    }

                    @if (bootstrap.state(); as bootstrapState) {
                      @if (bootstrapState.status === 'ready') {
                        <span class="system-readout" aria-hidden="true">
                          <span data-testid="engine-revision">{{
                            bootstrapState.value.rustyEngineRevision
                          }}</span>
                          <span data-testid="procgen-revision">{{
                            bootstrapState.value.rustyProcgenRevision
                          }}</span>
                        </span>
                      }
                    }
                  </div>
                }
                @if (gameMenuOpen()) {
                  <div class="game-menu-layer">
                    <div
                      class="game-menu-backdrop"
                      role="presentation"
                      (click)="closeGameMenu()"
                    ></div>
                    <section
                      #gameMenuPanel
                      class="panel game-menu"
                      role="dialog"
                      aria-modal="true"
                      aria-labelledby="game-menu-heading"
                      tabindex="-1"
                      (keydown)="gameMenuKeydown($event)"
                    >
                      <header>
                        <h2 id="game-menu-heading">Game menu</h2>
                        <button
                          type="button"
                          aria-label="Close game menu"
                          (click)="closeGameMenu()"
                        >
                          ×
                        </button>
                      </header>
                      @if (session.commandError(); as failure) {
                        <p
                          id="game-menu-error"
                          class="game-menu-error"
                          role="alert"
                          aria-atomic="true"
                        >
                          <strong>Action failed.</strong>
                          @if (failure.code !== null) {
                            <span> {{ failure.code }} ·</span>
                          }
                          <span> {{ failure.detail }}</span>
                        </p>
                      }
                      <div class="game-menu-actions">
                        <button
                          type="button"
                          [disabled]="session.busy()"
                          (click)="restartSession()"
                        >
                          New / Restart expedition
                        </button>
                        <button
                          type="button"
                          [disabled]="session.busy()"
                          (click)="saveSession()"
                        >
                          Save
                        </button>
                        <button
                          type="button"
                          [disabled]="session.busy()"
                          (click)="loadSession()"
                        >
                          Load saved session
                        </button>
                        <button
                          type="button"
                          disabled
                          aria-describedby="game-menu-exit-note"
                        >
                          Exit
                        </button>
                      </div>
                      <p id="game-menu-exit-note" class="game-menu-note">
                        Exit is reserved for native builds; browser sessions
                        stay open.
                      </p>
                    </section>
                  </div>
                }
              </section>
            }
          }
        }
      }
    </main>
  `,
})
export class GameShellComponent implements OnInit, OnDestroy {
  protected readonly bootstrap = inject(BootstrapStore);
  protected readonly session = inject(SessionStore);
  protected readonly selectedActionId = signal<string | null>(null);
  protected readonly drawer = signal<Drawer>(null);
  protected readonly gameMenuOpen = signal(false);
  protected readonly selectedAction = computed(() => {
    const state = this.session.state();
    if (state.status !== 'ready') {
      return null;
    }
    return (
      state.value.decision?.actions.find(
        (action) => action.actionId === this.selectedActionId(),
      ) ?? null
    );
  });
  private readonly rulesLog = viewChild<ElementRef<HTMLElement>>('rulesLog');
  private readonly partyTrigger =
    viewChild<ElementRef<HTMLButtonElement>>('partyTrigger');
  private readonly inventoryTrigger =
    viewChild<ElementRef<HTMLButtonElement>>('inventoryTrigger');
  private readonly gameMenuTrigger =
    viewChild<ElementRef<HTMLButtonElement>>('gameMenuTrigger');
  private readonly gameMenuPanel =
    viewChild<ElementRef<HTMLElement>>('gameMenuPanel');
  private restoreDrawerFocus: Exclude<Drawer, null> | null = null;
  private focusGameMenuOnOpen = false;
  private restoreGameMenuFocus = false;
  private stopKeyboard: (() => void) | null = null;

  constructor() {
    effect(() => {
      const state = this.session.state();
      if (
        state.status === 'ready' &&
        this.selectedActionId() !== null &&
        !state.value.decision?.actions.some(
          (action) => action.actionId === this.selectedActionId(),
        )
      ) {
        this.selectedActionId.set(null);
      }
    });
    afterRenderEffect(() => {
      this.session.log();
      const panel = this.rulesLog()?.nativeElement;
      if (panel !== undefined) {
        panel.scrollTop = panel.scrollHeight;
      }
      if (this.gameMenuOpen() && this.focusGameMenuOnOpen) {
        this.focusGameMenuOnOpen = false;
        this.gameMenuPanel()
          ?.nativeElement.querySelector<HTMLElement>('button:not(:disabled)')
          ?.focus();
      }
      if (!this.gameMenuOpen() && this.restoreGameMenuFocus) {
        this.restoreGameMenuFocus = false;
        this.gameMenuTrigger()?.nativeElement.focus();
      }
      const restore = this.restoreDrawerFocus;
      if (this.drawer() === null && restore !== null) {
        this.restoreDrawerFocus = null;
        (restore === 'party'
          ? this.partyTrigger()
          : this.inventoryTrigger()
        )?.nativeElement.focus();
      }
    });
  }

  ngOnInit(): void {
    void this.bootstrap.load();
    void this.session.load();
    this.stopKeyboard = observeGlobalKeydown((event) => this.keydown(event));
  }

  ngOnDestroy(): void {
    this.stopKeyboard?.();
    this.stopKeyboard = null;
  }

  protected reload(): void {
    void this.session.load();
  }

  protected async saveSession(): Promise<void> {
    if (await this.session.save()) {
      this.closeGameMenu();
    }
  }

  protected async loadSession(): Promise<void> {
    this.drawer.set(null);
    this.selectedActionId.set(null);
    if (await this.session.reopen()) {
      this.closeGameMenu();
    }
  }

  protected async restartSession(): Promise<void> {
    this.drawer.set(null);
    this.selectedActionId.set(null);
    if (await this.session.restart()) {
      this.closeGameMenu();
    }
  }

  protected openGameMenu(): void {
    this.restoreGameMenuFocus = false;
    this.restoreDrawerFocus = null;
    this.drawer.set(null);
    this.focusGameMenuOnOpen = true;
    this.gameMenuOpen.set(true);
  }

  protected closeGameMenu(): void {
    if (!this.gameMenuOpen()) {
      return;
    }
    this.restoreGameMenuFocus = true;
    this.gameMenuOpen.set(false);
  }

  protected gameMenuKeydown(event: KeyboardEvent): void {
    if (event.key !== 'Tab') {
      return;
    }
    const panel = event.currentTarget as HTMLElement;
    const buttons = Array.from(
      panel.querySelectorAll<HTMLButtonElement>('button:not(:disabled)'),
    );
    const first = buttons[0];
    const last = buttons.at(-1);
    if (first === undefined || last === undefined) {
      return;
    }
    if (event.shiftKey && event.target === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && event.target === last) {
      event.preventDefault();
      first.focus();
    }
  }

  protected openDrawer(drawer: Exclude<Drawer, null>): void {
    this.restoreDrawerFocus = null;
    this.drawer.set(drawer);
  }

  protected closeDrawer(): void {
    this.restoreDrawerFocus = this.drawer();
    this.drawer.set(null);
  }

  protected selectAction(action: LegalActionView): void {
    this.selectedActionId.set(
      this.selectedActionId() === action.actionId ? null : action.actionId,
    );
  }

  protected canStep(step: RelativeStep): boolean {
    const state = this.session.state();
    return (
      state.status === 'ready' &&
      (state.value.decision?.legalSteps.includes(step) ?? false)
    );
  }

  protected step(step: RelativeStep): void {
    const decision = this.decision();
    if (decision === null || !decision.legalSteps.includes(step)) {
      return;
    }
    void this.dispatch({
      kind: 'step',
      actorEntityId: decision.actorEntityId,
      expectedRevision: decision.expectedRevision,
      step,
    });
  }

  protected turn(direction: 'left' | 'right'): void {
    const decision = this.decision();
    if (decision === null || !decision.canTurn) {
      return;
    }
    void this.dispatch({
      kind: direction === 'left' ? 'turnLeft' : 'turnRight',
      actorEntityId: decision.actorEntityId,
      expectedRevision: decision.expectedRevision,
    });
  }

  protected wait(): void {
    const decision = this.decision();
    if (decision === null || !decision.canWait || this.session.busy()) {
      return;
    }
    void this.dispatch({
      kind: 'wait',
      actorEntityId: decision.actorEntityId,
      expectedRevision: decision.expectedRevision,
    });
  }

  protected useAction(action: LegalActionView, targetEntityId: number): void {
    const decision = this.decision();
    if (
      decision === null ||
      !decision.actions.some(
        (legal) =>
          legal.actionId === action.actionId &&
          legal.legalTargetEntityIds.includes(targetEntityId),
      )
    ) {
      return;
    }
    void this.dispatch({
      kind: 'useAction',
      actorEntityId: decision.actorEntityId,
      expectedRevision: decision.expectedRevision,
      actionId: action.actionId,
      targetEntityId,
    });
  }

  protected pickTarget(targetEntityId: number): void {
    const action = this.selectedAction();
    if (action !== null) {
      this.useAction(action, targetEntityId);
    }
  }

  protected targetName(entityId: number): string {
    const state = this.session.state();
    if (state.status !== 'ready') {
      return `Target ${entityId}`;
    }
    return (
      state.value.party.find((member) => member.entityId === entityId)?.name ??
      state.value.world.visibleActors.find(
        (actor) => actor.entityId === entityId,
      )?.name ??
      `Target ${entityId}`
    );
  }

  protected receiptSummary(entry: RulesLogEntry): string {
    const receipt = entry.receipt;
    switch (receipt.kind) {
      case 'partyMoved':
        return `R${entry.revision} · Party stepped ${receipt.step}`;
      case 'partyTurned':
        return `R${entry.revision} · Party turned ${receipt.direction}`;
      case 'partyWaited':
        return `R${entry.revision} · Party waited`;
      case 'partyAttacked':
        return `R${entry.revision} · ${receipt.hit ? 'Hit' : 'Miss'} with ${receipt.actionId}`;
      case 'oppositionAttacked':
        return `R${entry.revision} · ${this.targetName(receipt.target.selectedMemberEntityId)} was targeted`;
      case 'oppositionMoved':
        return `R${entry.revision} · Opposition advanced`;
      case 'oppositionPassed':
        return `R${entry.revision} · Opposition passed`;
      case 'loadoutMoved':
        return `R${entry.revision} · Loadout updated`;
      case 'expeditionBegan':
        return `R${entry.revision} · Expedition began`;
    }
  }

  protected receiptDetail(entry: RulesLogEntry): string {
    const receipt = entry.receipt;
    switch (receipt.kind) {
      case 'partyMoved':
        return `Actor ${receipt.actorEntityId}; accepted relative step ${receipt.step}.`;
      case 'partyTurned':
        return `Actor ${receipt.actorEntityId}; accepted ${receipt.direction} rotation.`;
      case 'partyWaited':
        return `Actor ${receipt.actorEntityId}; deliberately consumed one activation without movement or a roll.`;
      case 'partyAttacked':
        return attackDetail(receipt, `target ${receipt.targetEntityId}`);
      case 'oppositionAttacked':
        return `${attackDetail(receipt, `party member ${receipt.target.selectedMemberEntityId}`)} Selection ${receipt.target.selectionPolicy} over ${receipt.target.eligibleMemberCount} living members.`;
      case 'oppositionMoved':
        return `Actor ${receipt.actorEntityId}; one Engine-routed grid step.`;
      case 'oppositionPassed':
        return `Actor ${receipt.actorEntityId}; no legal attack or movement.`;
      case 'loadoutMoved':
        return `Item ${receipt.itemEntityId}; owner ${receipt.fromOwnerEntityId} to owner ${receipt.toOwnerEntityId}${receipt.destinationSlotId === null ? ' pack' : ` slot ${receipt.destinationSlotId}`}.`;
      case 'expeditionBegan':
        return 'Preparation completed; Rust admitted the expedition and settled to the first party decision.';
    }
  }

  private decision() {
    const state = this.session.state();
    return state.status === 'ready' ? state.value.decision : null;
  }

  private async dispatch(command: SessionCommandDto): Promise<void> {
    if (await this.session.command(command)) {
      this.selectedActionId.set(null);
    }
  }

  private keydown(event: KeyboardEvent): void {
    if (this.gameMenuOpen()) {
      if (event.key === 'Escape') {
        event.preventDefault();
        this.closeGameMenu();
      }
      return;
    }
    if (this.drawer() !== null) {
      if (event.key === 'Escape') {
        event.preventDefault();
        this.closeDrawer();
      }
      return;
    }
    if (
      event.defaultPrevented ||
      event.altKey ||
      event.ctrlKey ||
      event.metaKey
    ) {
      return;
    }
    if (keyboardEventTargetsEditable(event)) {
      return;
    }
    const key = event.key.toLowerCase();
    if (key === ' ' || key === 'spacebar') {
      if (event.repeat) {
        return;
      }
      if (keyboardEventTargetsInteractive(event)) {
        return;
      }
      const decision = this.decision();
      if (decision !== null && decision.canWait && !this.session.busy()) {
        event.preventDefault();
        this.wait();
      }
      return;
    }
    const step = new Map<string, RelativeStep>([
      ['arrowup', 'forward'],
      ['w', 'forward'],
      ['arrowdown', 'backward'],
      ['s', 'backward'],
      ['a', 'left'],
      ['d', 'right'],
    ]).get(key);
    if (step !== undefined) {
      event.preventDefault();
      this.step(step);
      return;
    }
    if (key === 'q' || key === 'e') {
      event.preventDefault();
      this.turn(key === 'q' ? 'left' : 'right');
      return;
    }
    const actionIndex = Number.parseInt(key, 10) - 1;
    const state = this.session.state();
    const action =
      state.status === 'ready'
        ? state.value.decision?.actions[actionIndex]
        : undefined;
    if (action !== undefined && action.legalTargetEntityIds.length > 0) {
      event.preventDefault();
      this.selectAction(action);
    }
  }
}

function attackDetail(
  receipt: Extract<
    TurnReceipt,
    { kind: 'partyAttacked' | 'oppositionAttacked' }
  >,
  target: string,
): string {
  const signedModifier =
    receipt.abilityModifier >= 0
      ? `+${receipt.abilityModifier}`
      : String(receipt.abilityModifier);
  return `${receipt.actionId} against ${target}: d20 ${receipt.d20} ${signedModifier} = ${receipt.attackTotal} vs defense ${receipt.defense}; ${receipt.hit ? 'hit' : 'miss'}; damage [${receipt.damageRolls.join(', ')}] ${receipt.damageBonus >= 0 ? '+' : ''}${receipt.damageBonus}, requested ${receipt.requestedDamage}, applied ${receipt.appliedDamage}.`;
}
