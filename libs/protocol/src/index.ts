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
  type CarriedItemView,
  type LegalActionView,
  type PartyDecisionView,
  type PartyMemberStatusView,
  type SessionErrorDto,
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
  'decision',
  'latestReceipts',
  'order',
  'outcome',
  'party',
  'revision',
  'round',
  'schemaVersion',
  'world',
] as const;
const SESSION_ERROR_KEYS = ['code', 'detail'] as const;
const ACTIVATION_KEYS = [
  'actorId',
  'entityId',
  'initiative',
  'name',
  'side',
] as const;
const PARTY_STATUS_KEYS = [
  'actorId',
  'carriedItems',
  'conscious',
  'currentVitality',
  'entityId',
  'maximumVitality',
  'name',
] as const;
const CARRIED_ITEM_KEYS = ['itemId', 'name'] as const;
const PARTY_DECISION_KEYS = [
  'actions',
  'actorEntityId',
  'canTurn',
  'expectedRevision',
  'legalSteps',
] as const;
const LEGAL_ACTION_KEYS = ['actionId', 'legalTargetEntityIds', 'name'] as const;
const SIMPLE_RECEIPT_KEYS = ['actorEntityId', 'kind'] as const;
const PARTY_MOVED_RECEIPT_KEYS = ['actorEntityId', 'kind', 'step'] as const;
const PARTY_TURNED_RECEIPT_KEYS = [
  'actorEntityId',
  'direction',
  'kind',
] as const;
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
    !Array.isArray(value['party']) ||
    value['party'].length < 1 ||
    value['party'].length > SESSION_VIEW_LIMITS.maxActivations
  ) {
    throw new Error('party status is not a bounded array');
  }
  const partyIds = new Set<number>();
  for (const member of value['party']) {
    decodePartyMemberStatus(member);
    if (partyIds.has(member.entityId)) {
      throw new Error('party status contains duplicate entities');
    }
    partyIds.add(member.entityId);
  }
  for (const activation of value['order']) {
    const member = value['party'].find(
      (candidate) => candidate.entityId === activation.entityId,
    );
    if (
      (activation.side === 'party' &&
        (member === undefined ||
          member.actorId !== activation.actorId ||
          member.name !== activation.name)) ||
      (activation.side === 'opposition' && member !== undefined)
    ) {
      throw new Error('activation side disagrees with party identity');
    }
  }
  if (value['outcome'] === 'ongoing') {
    const decision = value['decision'];
    const current = value['current'];
    decodePartyDecision(decision, value['world']);
    decodeActivation(current);
    if (
      decision.expectedRevision !== value['revision'] ||
      decision.actorEntityId !== current.entityId ||
      current.side !== 'party'
    ) {
      throw new Error('party decision does not match the current activation');
    }
  } else if (value['decision'] !== null) {
    throw new Error('terminal session exposes a party decision');
  }
  if (
    !Array.isArray(value['latestReceipts']) ||
    value['latestReceipts'].length > SESSION_VIEW_LIMITS.maxReceipts
  ) {
    throw new Error('turn receipts are not a bounded array');
  }
  for (const receipt of value['latestReceipts']) {
    decodeTurnReceipt(receipt);
    if (
      (receipt.kind.startsWith('party') &&
        !partyIds.has(receipt.actorEntityId)) ||
      (receipt.kind.startsWith('opposition') &&
        partyIds.has(receipt.actorEntityId)) ||
      (receipt.kind === 'oppositionAttacked' &&
        !partyIds.has(receipt.target.selectedMemberEntityId))
    ) {
      throw new Error('turn receipt disagrees with party identity');
    }
  }
  decodeWorldView(value['world']);
  return value as SessionView;
}

export function decodeSessionError(value: unknown): SessionErrorDto {
  requireExactRecord(value, SESSION_ERROR_KEYS, 'session error');
  requireId(value['code'], 'session error code');
  requireBoundedText(
    value['detail'],
    1,
    ROGUELIKE_LIMITS.maxAuthoredTextBytes,
    'session error detail',
  );
  return value as SessionErrorDto;
}

function decodePartyMemberStatus(
  value: unknown,
): asserts value is PartyMemberStatusView {
  requireExactRecord(value, PARTY_STATUS_KEYS, 'party member status');
  requireEntityId(value['entityId'], 'party member identity');
  requireId(value['actorId'], 'party member actor identity');
  requireBoundedText(
    value['name'],
    1,
    ROGUELIKE_LIMITS.maxAuthoredTextBytes,
    'party member name',
  );
  requireSafeInteger(value['currentVitality'], 0, 65_535, 'current vitality');
  requireSafeInteger(value['maximumVitality'], 1, 65_535, 'maximum vitality');
  if (Number(value['currentVitality']) > Number(value['maximumVitality'])) {
    throw new Error('party vitality exceeds its maximum');
  }
  if (value['conscious'] !== Number(value['currentVitality']) > 0) {
    throw new Error('party consciousness disagrees with vitality');
  }
  if (
    !Array.isArray(value['carriedItems']) ||
    value['carriedItems'].length > ROGUELIKE_LIMITS.maxDefinitionsPerKind
  ) {
    throw new Error('carried items are not a bounded array');
  }
  const itemIds = new Set<string>();
  for (const item of value['carriedItems']) {
    decodeCarriedItem(item);
    if (itemIds.has(item.itemId)) {
      throw new Error('party member status contains duplicate carried items');
    }
    itemIds.add(item.itemId);
  }
}

function decodeCarriedItem(value: unknown): asserts value is CarriedItemView {
  requireExactRecord(value, CARRIED_ITEM_KEYS, 'carried item');
  requireId(value['itemId'], 'carried item identity');
  requireBoundedText(
    value['name'],
    1,
    ROGUELIKE_LIMITS.maxAuthoredTextBytes,
    'carried item name',
  );
}

function decodePartyDecision(
  value: unknown,
  world: unknown,
): asserts value is PartyDecisionView {
  requireExactRecord(value, PARTY_DECISION_KEYS, 'party decision');
  requireEntityId(value['actorEntityId'], 'decision actor identity');
  requireSafeInteger(
    value['expectedRevision'],
    0,
    Number.MAX_SAFE_INTEGER,
    'decision revision',
  );
  if (typeof value['canTurn'] !== 'boolean') {
    throw new Error('party decision has an invalid turn fact');
  }
  if (!Array.isArray(value['legalSteps']) || value['legalSteps'].length > 4) {
    throw new Error('legal steps are not a bounded array');
  }
  const steps = new Set<string>();
  for (const step of value['legalSteps']) {
    if (!['forward', 'backward', 'left', 'right'].includes(String(step))) {
      throw new Error('party decision contains an invalid step');
    }
    if (steps.has(String(step))) {
      throw new Error('party decision contains duplicate steps');
    }
    steps.add(String(step));
  }
  if (
    !Array.isArray(value['actions']) ||
    value['actions'].length > ROGUELIKE_LIMITS.maxDefinitionsPerKind
  ) {
    throw new Error('legal actions are not a bounded array');
  }
  const decodedWorld = decodeWorldView(world);
  const visibleTargets = new Set(
    decodedWorld.visibleActors.map((actor) => actor.entityId),
  );
  const actionIds = new Set<string>();
  for (const action of value['actions']) {
    decodeLegalAction(action, visibleTargets);
    if (actionIds.has(action.actionId)) {
      throw new Error('party decision contains duplicate actions');
    }
    actionIds.add(action.actionId);
  }
}

function decodeLegalAction(
  value: unknown,
  visibleTargets: ReadonlySet<number>,
): asserts value is LegalActionView {
  requireExactRecord(value, LEGAL_ACTION_KEYS, 'legal action');
  requireId(value['actionId'], 'legal action identity');
  requireBoundedText(
    value['name'],
    1,
    ROGUELIKE_LIMITS.maxAuthoredTextBytes,
    'legal action name',
  );
  if (
    !Array.isArray(value['legalTargetEntityIds']) ||
    value['legalTargetEntityIds'].length > WORLD_VIEW_LIMITS.maxVisibleActors
  ) {
    throw new Error('legal targets are not a bounded array');
  }
  const targetIds = new Set<number>();
  for (const target of value['legalTargetEntityIds']) {
    requireEntityId(target, 'legal target identity');
    if (!visibleTargets.has(target)) {
      throw new Error('legal action references a nonvisible target');
    }
    if (targetIds.has(target)) {
      throw new Error('legal action contains duplicate targets');
    }
    targetIds.add(target);
  }
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
      requireExactRecord(
        value,
        PARTY_MOVED_RECEIPT_KEYS,
        'party movement receipt',
      );
      requireEntityId(value['actorEntityId'], 'receipt actor identity');
      if (
        !['forward', 'backward', 'left', 'right'].includes(
          String(value['step']),
        )
      ) {
        throw new Error('party movement receipt has an invalid step');
      }
      return;
    case 'partyTurned':
      requireExactRecord(
        value,
        PARTY_TURNED_RECEIPT_KEYS,
        'party turn receipt',
      );
      requireEntityId(value['actorEntityId'], 'receipt actor identity');
      if (value['direction'] !== 'left' && value['direction'] !== 'right') {
        throw new Error('party turn receipt has an invalid direction');
      }
      return;
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
