export * from './generated/api-types';

import {
  BOOTSTRAP_SCHEMA_VERSION,
  ROGUELIKE_ID_PATTERN,
  ROGUELIKE_LIMITS,
  RUSTY_ENGINE_REVISION,
  RUSTY_PROCGEN_REVISION,
  WORLD_VIEW_SCHEMA_VERSION,
  WORLD_VIEW_LIMITS,
  type BootstrapReadoutDto,
  type VisibleActorView,
  type WorldView,
  type WorldViewCell,
} from './generated/api-types';

const BOOTSTRAP_KEYS = [
  'phase',
  'procgenLinkHash',
  'product',
  'rustyEngineRevision',
  'rustyProcgenRevision',
  'schemaVersion',
] as const;

export function decodeBootstrapReadout(value: unknown): BootstrapReadoutDto {
  if (!isRecord(value)) {
    throw new Error('bootstrap response must be an object');
  }
  const keys = Object.keys(value).sort();
  if (
    keys.length !== BOOTSTRAP_KEYS.length ||
    keys.some((key, index) => key !== BOOTSTRAP_KEYS[index])
  ) {
    throw new Error('bootstrap response contains missing or unknown fields');
  }
  if (value['schemaVersion'] !== BOOTSTRAP_SCHEMA_VERSION) {
    throw new Error('bootstrap response has an unsupported schema');
  }
  if (
    value['product'] !== 'rusty-roguelike' ||
    value['phase'] !== 'bootstrap'
  ) {
    throw new Error('bootstrap response has the wrong product identity');
  }
  if (value['rustyEngineRevision'] !== RUSTY_ENGINE_REVISION) {
    throw new Error('bootstrap response has the wrong Rusty Engine revision');
  }
  if (value['rustyProcgenRevision'] !== RUSTY_PROCGEN_REVISION) {
    throw new Error('bootstrap response has the wrong Rusty Procgen revision');
  }
  if (
    typeof value['procgenLinkHash'] !== 'string' ||
    !/^fnv1a64:[0-9a-f]{16}$/.test(value['procgenLinkHash'])
  ) {
    throw new Error('bootstrap response has an invalid Procgen linkage hash');
  }
  return value as BootstrapReadoutDto;
}

const WORLD_VIEW_KEYS = [
  'cells',
  'discoveredCellCount',
  'facing',
  'floorId',
  'revision',
  'schemaVersion',
  'visibleActors',
] as const;
const WORLD_CELL_KEYS = ['depth', 'kind', 'lateral'] as const;
const VISIBLE_ACTOR_KEYS = [
  'actorId',
  'depth',
  'entityId',
  'lateral',
  'name',
  'participating',
] as const;
const ID_PATTERN = new RegExp(ROGUELIKE_ID_PATTERN);

export function decodeWorldView(value: unknown): WorldView {
  requireExactRecord(value, WORLD_VIEW_KEYS, 'world view');
  if (value['schemaVersion'] !== WORLD_VIEW_SCHEMA_VERSION) {
    throw new Error('world view has an unsupported schema');
  }
  requireSafeInteger(value['revision'], 0, Number.MAX_SAFE_INTEGER, 'revision');
  requireBoundedText(
    value['floorId'],
    1,
    WORLD_VIEW_LIMITS.maxFloorIdBytes,
    'floor identity',
  );
  if (
    typeof value['facing'] !== 'string' ||
    !['north', 'east', 'south', 'west'].includes(value['facing'])
  ) {
    throw new Error('world view has an invalid facing');
  }
  requireSafeInteger(
    value['discoveredCellCount'],
    0,
    WORLD_VIEW_LIMITS.maxDiscoveredCells,
    'discovery count',
  );
  if (
    !Array.isArray(value['cells']) ||
    value['cells'].length > WORLD_VIEW_LIMITS.maxProjectedFacts
  ) {
    throw new Error('world view cells are not a bounded array');
  }
  const cellKeys = new Set<string>();
  for (const cell of value['cells']) {
    decodeWorldCell(cell);
    const key = `${cell.lateral}:${cell.depth}`;
    if (cellKeys.has(key)) {
      throw new Error('world view contains duplicate cells');
    }
    cellKeys.add(key);
  }
  for (const wall of value['cells'].filter((cell) => cell.kind === 'wall')) {
    if (
      wall.depth > 0 &&
      value['cells'].some(
        (cell) =>
          cell.depth > wall.depth &&
          cell.lateral * wall.depth === wall.lateral * cell.depth,
      )
    ) {
      throw new Error('world view contains facts behind an occluding wall');
    }
  }
  if (
    !Array.isArray(value['visibleActors']) ||
    value['visibleActors'].length > WORLD_VIEW_LIMITS.maxVisibleActors
  ) {
    throw new Error('visible actors are not a bounded array');
  }
  const entityIds = new Set<number>();
  for (const actor of value['visibleActors']) {
    decodeVisibleActor(actor);
    if (entityIds.has(actor.entityId)) {
      throw new Error('world view contains duplicate visible actors');
    }
    entityIds.add(actor.entityId);
  }
  return value as WorldView;
}

function decodeWorldCell(value: unknown): asserts value is WorldViewCell {
  requireExactRecord(value, WORLD_CELL_KEYS, 'world view cell');
  requireRelativePosition(value);
  if (value['kind'] !== 'floor' && value['kind'] !== 'wall') {
    throw new Error('world view cell has an invalid kind');
  }
}

function decodeVisibleActor(value: unknown): asserts value is VisibleActorView {
  requireExactRecord(value, VISIBLE_ACTOR_KEYS, 'visible actor');
  requireRelativePosition(value);
  requireSafeInteger(
    value['entityId'],
    1,
    Number.MAX_SAFE_INTEGER,
    'entity identity',
  );
  requireBoundedText(
    value['actorId'],
    1,
    ROGUELIKE_LIMITS.maxIdBytes,
    'actor identity',
  );
  if (!ID_PATTERN.test(value['actorId'])) {
    throw new Error('visible actor has an invalid actor identity');
  }
  requireBoundedText(
    value['name'],
    1,
    ROGUELIKE_LIMITS.maxAuthoredTextBytes,
    'actor name',
  );
  if (value['participating'] !== true) {
    throw new Error('visible actor has an invalid participation fact');
  }
}

function requireRelativePosition(value: Record<string, unknown>): void {
  requireSafeInteger(
    value['depth'],
    0,
    WORLD_VIEW_LIMITS.maxDepth,
    'relative depth',
  );
  requireSafeInteger(
    value['lateral'],
    -WORLD_VIEW_LIMITS.maxDepth,
    WORLD_VIEW_LIMITS.maxDepth,
    'relative lateral position',
  );
  const maximumLateral = Math.max(Number(value['depth']), 1);
  if (Math.abs(Number(value['lateral'])) > maximumLateral) {
    throw new Error('world view fact falls outside the bounded view cone');
  }
}

function requireExactRecord<const Keys extends readonly string[]>(
  value: unknown,
  expected: Keys,
  label: string,
): asserts value is Record<Keys[number], unknown> {
  if (!isRecord(value)) {
    throw new Error(`${label} must be an object`);
  }
  const keys = Object.keys(value).sort();
  if (
    keys.length !== expected.length ||
    keys.some((key, index) => key !== expected[index])
  ) {
    throw new Error(`${label} contains missing or unknown fields`);
  }
}

function requireSafeInteger(
  value: unknown,
  minimum: number,
  maximum: number,
  label: string,
): void {
  if (
    typeof value !== 'number' ||
    !Number.isSafeInteger(value) ||
    value < minimum ||
    value > maximum
  ) {
    throw new Error(`${label} is outside its accepted integer range`);
  }
}

function requireBoundedText(
  value: unknown,
  minimum: number,
  maximum: number,
  label: string,
): asserts value is string {
  if (
    typeof value !== 'string' ||
    value.length < minimum ||
    value.length > maximum ||
    Array.from(value).some((character) => {
      const code = character.charCodeAt(0);
      return code < 32 || code === 127;
    })
  ) {
    throw new Error(`${label} is invalid`);
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
