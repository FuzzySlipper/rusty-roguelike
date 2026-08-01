export * from './generated/api-types';

import {
  BOOTSTRAP_SCHEMA_VERSION,
  ROGUELIKE_ID_PATTERN,
  ROGUELIKE_LIMITS,
  RUSTY_ENGINE_REVISION,
  RUSTY_PROCGEN_REVISION,
  SESSION_VIEW_LIMITS,
  SESSION_VIEW_SCHEMA_VERSION,
  WORLD_VIEW_SCHEMA_VERSION,
  WORLD_VIEW_LIMITS,
  type ActivationView,
  type BootstrapReadoutDto,
  type SessionView,
  type TurnReceipt,
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
    if (
      !value['cells'].some(
        (cell) =>
          cell.kind === 'floor' &&
          cell.lateral === actor.lateral &&
          cell.depth === actor.depth,
      )
    ) {
      throw new Error('visible actor does not occupy a projected floor fact');
    }
    if (entityIds.has(actor.entityId)) {
      throw new Error('world view contains duplicate visible actors');
    }
    entityIds.add(actor.entityId);
  }
  return value as WorldView;
}

const SESSION_VIEW_KEYS = [
  'current',
  'latestReceipts',
  'order',
  'outcome',
  'revision',
  'round',
  'schemaVersion',
  'world',
] as const;
const ACTIVATION_KEYS = [
  'actorId',
  'entityId',
  'initiative',
  'name',
  'side',
] as const;
const SIMPLE_RECEIPT_KEYS = ['actorEntityId', 'kind'] as const;
const PARTY_ATTACK_RECEIPT_KEYS = [
  'abilityModifier',
  'actionId',
  'actorEntityId',
  'appliedDamage',
  'attackTotal',
  'd20',
  'damageBonus',
  'damageRolls',
  'defense',
  'hit',
  'kind',
  'requestedDamage',
  'targetEntityId',
] as const;
const OPPOSITION_ATTACK_RECEIPT_KEYS = [
  'abilityModifier',
  'actionId',
  'actorEntityId',
  'appliedDamage',
  'attackTotal',
  'd20',
  'damageBonus',
  'damageRolls',
  'defense',
  'hit',
  'kind',
  'requestedDamage',
  'target',
] as const;
const PARTY_SQUARE_TARGET_KEYS = [
  'eligibleMemberCount',
  'selectedMemberEntityId',
  'selectionPolicy',
] as const;

export function decodeSessionView(value: unknown): SessionView {
  requireExactRecord(value, SESSION_VIEW_KEYS, 'session view');
  if (value['schemaVersion'] !== SESSION_VIEW_SCHEMA_VERSION) {
    throw new Error('session view has an unsupported schema');
  }
  requireSafeInteger(value['revision'], 0, Number.MAX_SAFE_INTEGER, 'revision');
  requireSafeInteger(value['round'], 1, Number.MAX_SAFE_INTEGER, 'round');
  if (!['ongoing', 'victory', 'defeat'].includes(String(value['outcome']))) {
    throw new Error('session view has an invalid outcome');
  }
  if (
    !Array.isArray(value['order']) ||
    value['order'].length > SESSION_VIEW_LIMITS.maxActivations
  ) {
    throw new Error('activation order is not a bounded array');
  }
  const activationIds = new Set<number>();
  for (const activation of value['order']) {
    decodeActivation(activation);
    if (activationIds.has(activation.entityId)) {
      throw new Error('activation order contains duplicate entities');
    }
    activationIds.add(activation.entityId);
  }
  if (value['outcome'] === 'ongoing') {
    const current = value['current'];
    decodeActivation(current);
    if (
      !value['order'].some(
        (activation) =>
          activation.entityId === current.entityId &&
          activation.actorId === current.actorId &&
          activation.name === current.name &&
          activation.side === current.side &&
          activation.initiative === current.initiative,
      )
    ) {
      throw new Error('current activation is absent from the activation order');
    }
  } else if (value['current'] !== null || value['order'].length !== 0) {
    throw new Error('terminal session exposes a live activation');
  }
  if (
    !Array.isArray(value['latestReceipts']) ||
    value['latestReceipts'].length > SESSION_VIEW_LIMITS.maxReceipts
  ) {
    throw new Error('turn receipts are not a bounded array');
  }
  for (const receipt of value['latestReceipts']) {
    decodeTurnReceipt(receipt);
  }
  decodeWorldView(value['world']);
  return value as SessionView;
}

function decodeActivation(value: unknown): asserts value is ActivationView {
  requireExactRecord(value, ACTIVATION_KEYS, 'activation');
  requireEntityId(value['entityId'], 'activation entity identity');
  requireId(value['actorId'], 'activation actor identity');
  requireBoundedText(
    value['name'],
    1,
    ROGUELIKE_LIMITS.maxAuthoredTextBytes,
    'activation name',
  );
  if (value['side'] !== 'party' && value['side'] !== 'opposition') {
    throw new Error('activation has an invalid side');
  }
  requireI16(value['initiative'], 'initiative');
}

function decodeTurnReceipt(value: unknown): asserts value is TurnReceipt {
  if (!isRecord(value) || typeof value['kind'] !== 'string') {
    throw new Error('turn receipt must be a tagged object');
  }
  switch (value['kind']) {
    case 'partyMoved':
    case 'partyTurned':
    case 'oppositionMoved':
    case 'oppositionPassed':
      requireExactRecord(value, SIMPLE_RECEIPT_KEYS, 'turn receipt');
      requireEntityId(value['actorEntityId'], 'receipt actor identity');
      return;
    case 'partyAttacked':
      requireExactRecord(
        value,
        PARTY_ATTACK_RECEIPT_KEYS,
        'party attack receipt',
      );
      requireEntityId(value['targetEntityId'], 'receipt target identity');
      decodeAttackReceipt(value);
      return;
    case 'oppositionAttacked':
      requireExactRecord(
        value,
        OPPOSITION_ATTACK_RECEIPT_KEYS,
        'opposition attack receipt',
      );
      decodePartySquareTarget(value['target']);
      decodeAttackReceipt(value);
      return;
    default:
      throw new Error('turn receipt has an unsupported kind');
  }
}

function decodeAttackReceipt(value: Record<string, unknown>): void {
  requireEntityId(value['actorEntityId'], 'receipt actor identity');
  requireId(value['actionId'], 'receipt action identity');
  requireSafeInteger(value['d20'], 1, 20, 'attack d20');
  requireI16(value['abilityModifier'], 'attack ability modifier');
  requireI16(value['attackTotal'], 'attack total');
  requireI16(value['defense'], 'attack defense');
  requireI16(value['damageBonus'], 'damage bonus');
  if (
    value['attackTotal'] !==
    Number(value['d20']) + Number(value['abilityModifier'])
  ) {
    throw new Error('attack receipt has inconsistent arithmetic');
  }
  if (typeof value['hit'] !== 'boolean') {
    throw new Error('attack receipt has an invalid hit fact');
  }
  if (
    !Array.isArray(value['damageRolls']) ||
    value['damageRolls'].length < 1 ||
    value['damageRolls'].length > ROGUELIKE_LIMITS.maxDamageDice
  ) {
    throw new Error('damage rolls are not a bounded array');
  }
  for (const roll of value['damageRolls']) {
    requireSafeInteger(
      roll,
      1,
      ROGUELIKE_LIMITS.maxDamageDieSides,
      'damage roll',
    );
  }
  requireSafeInteger(value['requestedDamage'], 0, 65_535, 'requested damage');
  requireSafeInteger(value['appliedDamage'], 0, 65_535, 'applied damage');
  if (Number(value['appliedDamage']) > Number(value['requestedDamage'])) {
    throw new Error('attack receipt applies more damage than requested');
  }
  if (
    value['hit'] === false &&
    (value['requestedDamage'] !== 0 || value['appliedDamage'] !== 0)
  ) {
    throw new Error('miss receipt contains applied damage');
  }
}

function decodePartySquareTarget(value: unknown): void {
  requireExactRecord(
    value,
    PARTY_SQUARE_TARGET_KEYS,
    'party-square target receipt',
  );
  requireEntityId(
    value['selectedMemberEntityId'],
    'selected party member identity',
  );
  if (value['selectionPolicy'] !== 'round-robin-living') {
    throw new Error('party-square target has an invalid selection policy');
  }
  requireSafeInteger(
    value['eligibleMemberCount'],
    1,
    SESSION_VIEW_LIMITS.maxActivations,
    'eligible party member count',
  );
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

function requireEntityId(value: unknown, label: string): void {
  requireSafeInteger(value, 1, Number.MAX_SAFE_INTEGER, label);
}

function requireI16(value: unknown, label: string): void {
  requireSafeInteger(value, -32_768, 32_767, label);
}

function requireId(value: unknown, label: string): asserts value is string {
  requireBoundedText(value, 1, ROGUELIKE_LIMITS.maxIdBytes, label);
  if (!ID_PATTERN.test(value)) {
    throw new Error(`${label} is invalid`);
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
