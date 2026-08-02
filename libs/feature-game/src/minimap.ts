import {
  ChangeDetectionStrategy,
  Component,
  computed,
  input,
} from '@angular/core';

import type {
  Facing,
  MinimapCellView,
  MinimapFeatureKind,
  MinimapView,
} from '@rusty-roguelike/protocol';

interface RenderedCell extends MinimapCellView {
  readonly key: string;
  readonly mapX: number;
  readonly mapY: number;
}

interface MinimapLayout {
  readonly cells: readonly RenderedCell[];
  readonly height: number;
  readonly partyX: number;
  readonly partyY: number;
  readonly visibleActors: readonly {
    readonly entityId: number;
    readonly mapX: number;
    readonly mapY: number;
    readonly name: string;
    readonly participating: boolean;
  }[];
  readonly width: number;
}

@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  selector: 'rr-minimap',
  standalone: true,
  styles: [
    `
      :host {
        display: block;
        min-width: 0;
        pointer-events: none;
      }

      figure {
        backdrop-filter: blur(12px);
        background: rgb(7 17 20 / 0.92);
        border: 1px solid var(--rr-line);
        border-radius: 9px;
        box-shadow: 0 12px 36px rgb(0 0 0 / 0.28);
        margin: 0;
        padding: 0.45rem;
        pointer-events: auto;
      }

      figure:focus-visible {
        border-color: var(--rr-accent);
        outline: 2px solid rgb(126 229 210 / 0.4);
        outline-offset: 2px;
      }

      header {
        align-items: baseline;
        display: flex;
        gap: 0.5rem;
        justify-content: space-between;
        margin-bottom: 0.3rem;
      }

      h2,
      p {
        margin: 0;
      }

      h2 {
        color: var(--rr-accent);
        font-size: 0.68rem;
        letter-spacing: 0.11em;
        text-transform: uppercase;
      }

      header p {
        color: var(--rr-muted);
        font-size: 0.64rem;
      }

      svg {
        background: rgb(2 8 10 / 0.84);
        border: 1px solid rgb(126 229 210 / 0.12);
        border-radius: 5px;
        display: block;
        max-height: min(29dvh, 260px);
        min-height: 104px;
        width: 100%;
      }

      .cell {
        stroke: rgb(1 5 7 / 0.9);
        stroke-width: 0.045;
      }

      .floor.visible {
        fill: #49605b;
      }

      .floor.remembered {
        fill: #263632;
      }

      .wall.visible {
        fill: #86938a;
      }

      .wall.remembered {
        fill: #48534e;
      }

      .feature,
      .enemy {
        dominant-baseline: middle;
        font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
        font-size: 0.68px;
        font-weight: 900;
        paint-order: stroke;
        stroke: rgb(1 5 7 / 0.9);
        stroke-width: 0.12px;
        text-anchor: middle;
      }

      .feature {
        fill: #f4c86b;
      }

      .enemy {
        fill: #ff837a;
      }

      .party {
        fill: #7ee5d2;
        stroke: rgb(3 15 17 / 0.95);
        stroke-linejoin: round;
        stroke-width: 0.09;
      }

      figcaption {
        color: var(--rr-muted);
        display: flex;
        flex-wrap: wrap;
        font-size: 0.61rem;
        gap: 0.2rem 0.5rem;
        margin-top: 0.35rem;
      }

      .legend-item {
        align-items: center;
        display: inline-flex;
        gap: 0.2rem;
      }

      .swatch {
        border: 1px solid rgb(255 255 255 / 0.2);
        display: inline-block;
        height: 0.55rem;
        width: 0.55rem;
      }

      .swatch.visible {
        background: #657a73;
      }

      .swatch.remembered {
        background: #2c3a36;
      }

      .glyph {
        color: #f4c86b;
        font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
        font-weight: 900;
      }

      .glyph.enemy {
        color: #ff837a;
      }

      .sr-only {
        clip: rect(0 0 0 0);
        clip-path: inset(50%);
        height: 1px;
        overflow: hidden;
        position: absolute;
        white-space: nowrap;
        width: 1px;
      }

      @media (max-width: 760px) {
        figure {
          padding: 0.35rem;
        }

        header p {
          display: none;
        }

        svg {
          max-height: 24dvh;
          min-height: 92px;
        }

        figcaption {
          gap: 0.15rem 0.35rem;
        }
      }
    `,
  ],
  template: `
    @if (layout(); as map) {
      <figure
        tabindex="0"
        role="img"
        [attr.aria-label]="summary()"
        aria-describedby="minimap-detail"
        [attr.data-minimap-revision]="revision()"
        [attr.data-discovered-cells]="minimap().cells.length"
        [attr.data-visible-cells]="visibleCellCount()"
        [attr.data-visible-enemies]="minimap().visibleActors.length"
      >
        <header aria-hidden="true">
          <h2>Floor map</h2>
          <p>{{ minimap().cells.length }} discovered cells</p>
        </header>
        <svg
          aria-hidden="true"
          preserveAspectRatio="xMidYMid meet"
          [attr.viewBox]="'0 0 ' + map.width + ' ' + map.height"
        >
          @for (cell of map.cells; track cell.key) {
            <rect
              class="cell"
              [class.floor]="cell.terrain === 'floor'"
              [class.wall]="cell.terrain === 'wall'"
              [class.visible]="cell.visible"
              [class.remembered]="!cell.visible"
              [attr.x]="cell.mapX"
              [attr.y]="cell.mapY"
              width="1"
              height="1"
            />
            @if (cell.feature; as feature) {
              <text
                class="feature"
                [attr.x]="cell.mapX + 0.5"
                [attr.y]="cell.mapY + 0.53"
              >
                {{ featureGlyph(feature) }}
              </text>
            }
          }
          @for (actor of map.visibleActors; track actor.entityId) {
            <text
              class="enemy"
              [attr.x]="actor.mapX + 0.5"
              [attr.y]="actor.mapY + 0.53"
            >
              !
            </text>
          }
          <g
            [attr.transform]="
              'translate(' +
              map.partyX +
              ' ' +
              map.partyY +
              ') rotate(' +
              facingRotation(minimap().facing) +
              ' .5 .5)'
            "
          >
            <path class="party" d="M .5 .1 L .86 .84 L .5 .69 L .14 .84 Z" />
          </g>
        </svg>
        <figcaption aria-hidden="true">
          <span class="legend-item"
            ><span class="swatch visible"></span>Visible</span
          >
          <span class="legend-item"
            ><span class="swatch remembered"></span>Memory</span
          >
          <span class="legend-item"><span class="glyph">◆</span>Feature</span>
          <span class="legend-item"
            ><span class="glyph enemy">!</span>Enemy</span
          >
        </figcaption>
        <span id="minimap-detail" class="sr-only">{{ detail() }}</span>
      </figure>
    }
  `,
})
export class MinimapComponent {
  readonly minimap = input.required<MinimapView>();
  readonly revision = input.required<number>();

  protected readonly layout = computed(() => layoutMinimap(this.minimap()));
  protected readonly summary = computed(() => {
    const map = this.minimap();
    const current = map.cells.filter((cell) => cell.visible).length;
    return `Floor map at revision ${this.revision()}. Party facing ${map.facing}. ${map.cells.length} discovered cells, ${current} currently visible, ${map.visibleActors.length} visible enemies.`;
  });
  protected readonly visibleCellCount = computed(
    () => this.minimap().cells.filter((cell) => cell.visible).length,
  );
  protected readonly detail = computed(() => minimapDetail(this.minimap()));

  protected facingRotation(facing: Facing): number {
    return facingRotation(facing);
  }

  protected featureGlyph(feature: MinimapFeatureKind): string {
    return featureGlyph(feature);
  }
}

export function layoutMinimap(minimap: MinimapView): MinimapLayout {
  const allX = [...minimap.cells.map((cell) => cell.x), minimap.party.x];
  const allY = [...minimap.cells.map((cell) => cell.y), minimap.party.y];
  const minimumX = Math.min(...allX);
  const maximumX = Math.max(...allX);
  const minimumY = Math.min(...allY);
  const maximumY = Math.max(...allY);
  const offsetX = 1 - minimumX;
  const offsetY = 1 - minimumY;
  return {
    cells: minimap.cells.map((cell) => ({
      ...cell,
      key: `${cell.x},${cell.y}`,
      mapX: cell.x + offsetX,
      mapY: cell.y + offsetY,
    })),
    height: maximumY - minimumY + 3,
    partyX: minimap.party.x + offsetX,
    partyY: minimap.party.y + offsetY,
    visibleActors: minimap.visibleActors.map((actor) => ({
      entityId: actor.entityId,
      mapX: actor.x + offsetX,
      mapY: actor.y + offsetY,
      name: actor.name,
      participating: actor.participating,
    })),
    width: maximumX - minimumX + 3,
  };
}

export function facingRotation(facing: Facing): number {
  return facing === 'north'
    ? 0
    : facing === 'east'
      ? 90
      : facing === 'south'
        ? 180
        : 270;
}

export function featureGlyph(feature: MinimapFeatureKind): string {
  switch (feature) {
    case 'entry':
      return '⌂';
    case 'gate':
      return '▣';
    case 'goal':
      return '◆';
    case 'key':
      return '⚿';
    case 'open-door':
      return '/';
    case 'locked-door':
      return '+';
  }
}

export function minimapDetail(minimap: MinimapView): string {
  const cells = minimap.cells.map((cell) => {
    const visibility = cell.visible ? 'visible' : 'remembered';
    const feature = cell.feature === null ? '' : ` with ${cell.feature}`;
    return `${visibility} ${cell.terrain}${feature} at ${cell.x}, ${cell.y}`;
  });
  const actors = minimap.visibleActors.map(
    (actor) =>
      `${actor.name}, ${actor.participating ? 'participating' : 'dormant'}, at ${actor.x}, ${actor.y}`,
  );
  return [
    `Party at ${minimap.party.x}, ${minimap.party.y}.`,
    ...cells,
    ...actors,
  ].join(' ');
}
