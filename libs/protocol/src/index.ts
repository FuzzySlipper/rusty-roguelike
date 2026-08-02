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
  type LoadoutItemView,
  type LoadoutView,
  type LegalActionView,
  type MinimapActorView,
  type MinimapCellView,
  type PartyDecisionView,
  type PartyMemberStatusView,
  type SessionErrorDto,
  type SessionLogEntry,
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
  'minimap',
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
const MINIMAP_KEYS = ['cells', 'facing', 'party', 'visibleActors'] as const;
const WORLD_POSITION_KEYS = ['x', 'y'] as const;
const MINIMAP_CELL_KEYS = ['feature', 'terrain', 'visible', 'x', 'y'] as const;
const MINIMAP_ACTOR_KEYS = [
  'actorId',
  'entityId',
  'name',
  'participating',
  'x',
  'y',
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
  decodeMinimap(value['minimap'], value as WorldView);
  return value as WorldView;
}

function decodeMinimap(value: unknown, world: WorldView): void {
  requireExactRecord(value, MINIMAP_KEYS, 'minimap');
  requireExactRecord(value['party'], WORLD_POSITION_KEYS, 'minimap party');
  requireCoordinate(value['party']['x'], 'minimap party x');
  requireCoordinate(value['party']['y'], 'minimap party y');
  const party = value['party'] as { x: number; y: number };
  if (value['facing'] !== world.facing) {
    throw new Error('minimap facing does not match the world view');
  }
  if (
    !Array.isArray(value['cells']) ||
    value['cells'].length > WORLD_VIEW_LIMITS.maxMinimapFacts
  ) {
    throw new Error('minimap cells are not a bounded array');
  }
  const cells = new Map<string, MinimapCellView>();
  for (const cell of value['cells']) {
    decodeMinimapCell(cell);
    const key = String(cell.x) + ':' + String(cell.y);
    if (cells.has(key)) {
      throw new Error('minimap contains duplicate cells');
    }
    cells.set(key, cell);
  }
  const partyKey =
    String(value['party']['x']) + ':' + String(value['party']['y']);
  const partyCell = cells.get(partyKey);
  if (partyCell?.terrain !== 'floor' || !partyCell.visible) {
    throw new Error('minimap party does not occupy a currently visible floor');
  }
  for (const cell of world.cells) {
    const absolute = absoluteCell(party, world.facing, cell);
    const minimapCell = cells.get(
      String(absolute.x) + ':' + String(absolute.y),
    );
    if (minimapCell?.terrain !== cell.kind || !minimapCell.visible) {
      throw new Error('minimap current terrain does not match the world view');
    }
  }
  if (
    [...cells.values()].filter((cell) => cell.visible).length !==
    world.cells.length
  ) {
    throw new Error('minimap current terrain does not match the world view');
  }
  if (
    !Array.isArray(value['visibleActors']) ||
    value['visibleActors'].length !== world.visibleActors.length
  ) {
    throw new Error('minimap visible actors do not match the world view');
  }
  const actors = new Map<number, MinimapActorView>();
  for (const actor of value['visibleActors']) {
    decodeMinimapActor(actor);
    const cell = cells.get(String(actor.x) + ':' + String(actor.y));
    if (
      cell?.terrain !== 'floor' ||
      !cell.visible ||
      actors.has(actor.entityId)
    ) {
      throw new Error('minimap actor does not occupy unique visible floor');
    }
    actors.set(actor.entityId, actor);
  }
  for (const actor of world.visibleActors) {
    const absolute = absoluteCell(party, world.facing, actor);
    const minimapActor = actors.get(actor.entityId);
    if (
      minimapActor?.actorId !== actor.actorId ||
      minimapActor.name !== actor.name ||
      minimapActor.participating !== actor.participating ||
      minimapActor.x !== absolute.x ||
      minimapActor.y !== absolute.y
    ) {
      throw new Error('minimap actor facts do not match the world view');
    }
  }
}

function decodeMinimapCell(value: unknown): asserts value is MinimapCellView {
  requireExactRecord(value, MINIMAP_CELL_KEYS, 'minimap cell');
  requireCoordinate(value['x'], 'minimap cell x');
  requireCoordinate(value['y'], 'minimap cell y');
  if (!['floor', 'wall'].includes(String(value['terrain']))) {
    throw new Error('minimap cell has invalid terrain');
  }
  if (
    value['feature'] !== null &&
    !['entry', 'goal', 'key', 'open-door', 'locked-door'].includes(
      String(value['feature']),
    )
  ) {
    throw new Error('minimap cell has invalid feature');
  }
  if (value['terrain'] === 'wall' && value['feature'] !== null) {
    throw new Error('minimap wall cannot carry a floor feature');
  }
  if (typeof value['visible'] !== 'boolean') {
    throw new Error('minimap cell visibility must be boolean');
  }
}

function decodeMinimapActor(value: unknown): asserts value is MinimapActorView {
  requireExactRecord(value, MINIMAP_ACTOR_KEYS, 'minimap actor');
  requireId(value['actorId'], 'minimap actor identity');
  requireSafeInteger(
    value['entityId'],
    1,
    Number.MAX_SAFE_INTEGER,
    'minimap actor entity',
  );
  requireBoundedText(value['name'], 1, 128, 'minimap actor name');
  requireCoordinate(value['x'], 'minimap actor x');
  requireCoordinate(value['y'], 'minimap actor y');
  if (typeof value['participating'] !== 'boolean') {
    throw new Error('minimap actor participation must be boolean');
  }
}

function absoluteCell(
  party: { x: number; y: number },
  facing: WorldView['facing'],
  relative: { lateral: number; depth: number },
): { x: number; y: number } {
  const [forwardX, forwardY, rightX, rightY] =
    facing === 'north'
      ? [0, -1, 1, 0]
      : facing === 'east'
        ? [1, 0, 0, 1]
        : facing === 'south'
          ? [0, 1, -1, 0]
          : [-1, 0, 0, -1];
  return {
    x: party.x + forwardX * relative.depth + rightX * relative.lateral,
    y: party.y + forwardY * relative.depth + rightY * relative.lateral,
  };
}

function requireCoordinate(value: unknown, label: string): void {
  requireSafeInteger(value, -2_147_483_648, 2_147_483_647, label);
}

const SESSION_VIEW_KEYS = [
  'current',
  'decision',
  'latestReceipts',
  'log',
  'order',
  'outcome',
  'party',
  'phase',
  'preparation',
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
  'abilities',
  'actions',
  'actorId',
  'classId',
  'classLevel',
  'className',
  'conscious',
  'currentVitality',
  'defenses',
  'entityId',
  'experience',
  'feats',
  'level',
  'loadout',
  'maximumVitality',
  'name',
  'title',
] as const;
const ABILITY_KEYS = ['abilityId', 'modifier', 'score'] as const;
const DEFENSE_KEYS = ['defenseId', 'value'] as const;
const FEAT_KEYS = ['description', 'featId', 'name'] as const;
const CHARACTER_ACTION_KEYS = ['actionId', 'name'] as const;
const LOADOUT_KEYS = [
  'capacity',
  'equipmentSlots',
  'inventorySlots',
  'ownerEntityId',
] as const;
const LOADOUT_ITEM_KEYS = [
  'entityId',
  'equipmentSlotId',
  'equippedSlotId',
  'itemId',
  'name',
] as const;
const LOADOUT_CAPACITY_KEYS = ['maximum', 'used'] as const;
const EQUIPMENT_SLOT_KEYS = ['equipped', 'label', 'slotId'] as const;
const PREPARATION_KEYS = ['ready', 'stash'] as const;
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
const LOADOUT_MOVED_RECEIPT_KEYS = [
  'destinationSlotId',
  'fromOwnerEntityId',
  'itemEntityId',
  'kind',
  'toOwnerEntityId',
] as const;
const EXPEDITION_BEGAN_RECEIPT_KEYS = ['kind'] as const;
const SESSION_LOG_ENTRY_KEYS = ['id', 'receipt', 'revision'] as const;

export function decodeSessionView(value: unknown): SessionView {
  requireExactRecord(value, SESSION_VIEW_KEYS, 'session view');
  if (value['schemaVersion'] !== SESSION_VIEW_SCHEMA_VERSION) {
    throw new Error('session view has an unsupported schema');
  }
  requireSafeInteger(value['revision'], 0, Number.MAX_SAFE_INTEGER, 'revision');
  requireSafeInteger(value['round'], 1, Number.MAX_SAFE_INTEGER, 'round');
  if (!['preparation', 'expedition'].includes(String(value['phase']))) {
    throw new Error('session view has an invalid phase');
  }
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
  if (value['phase'] === 'preparation') {
    if (
      value['outcome'] !== 'ongoing' ||
      value['current'] !== null ||
      value['order'].length !== 0 ||
      value['decision'] !== null
    ) {
      throw new Error('preparation exposes an expedition activation');
    }
  } else if (value['outcome'] === 'ongoing') {
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
  const loadoutItemIds = new Set<number>();
  let stashOwnerId: number | null = null;
  for (const member of value['party']) {
    decodePartyMemberStatus(member);
    if (partyIds.has(member.entityId)) {
      throw new Error('party status contains duplicate entities');
    }
    partyIds.add(member.entityId);
    if (member.loadout.ownerEntityId !== member.entityId) {
      throw new Error('party loadout owner disagrees with member identity');
    }
    for (const item of member.loadout.inventorySlots.filter(
      (candidate): candidate is LoadoutItemView => candidate !== null,
    )) {
      if (loadoutItemIds.has(item.entityId)) {
        throw new Error('party loadouts contain a duplicate item entity');
      }
      loadoutItemIds.add(item.entityId);
    }
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
  if (value['phase'] === 'expedition' && value['outcome'] === 'ongoing') {
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
  if (value['phase'] === 'preparation') {
    requireExactRecord(value['preparation'], PREPARATION_KEYS, 'preparation');
    decodeLoadout(value['preparation']['stash'], false);
    if (typeof value['preparation']['ready'] !== 'boolean') {
      throw new Error('preparation has an invalid ready fact');
    }
    const stash = value['preparation']['stash'];
    stashOwnerId = stash.ownerEntityId;
    if (partyIds.has(stash.ownerEntityId)) {
      throw new Error('shared stash reuses a party owner');
    }
    for (const item of stash.inventorySlots.filter(
      (candidate): candidate is LoadoutItemView => candidate !== null,
    )) {
      if (loadoutItemIds.has(item.entityId)) {
        throw new Error('shared stash duplicates a party item entity');
      }
      loadoutItemIds.add(item.entityId);
    }
    const allPartyItemsEquipped = (
      value['party'] as PartyMemberStatusView[]
    ).every((member) =>
      member.loadout.inventorySlots
        .filter((candidate): candidate is LoadoutItemView => candidate !== null)
        .every((item) => item.equippedSlotId !== null),
    );
    const expectedReady = stash.capacity.used === 0 && allPartyItemsEquipped;
    if (value['preparation']['ready'] !== expectedReady) {
      throw new Error('preparation ready fact disagrees with loadout state');
    }
  } else if (value['preparation'] !== null) {
    throw new Error('expedition exposes the remote preparation stash');
  }
  if (
    !Array.isArray(value['latestReceipts']) ||
    value['latestReceipts'].length > SESSION_VIEW_LIMITS.maxReceipts
  ) {
    throw new Error('turn receipts are not a bounded array');
  }
  for (const receipt of value['latestReceipts']) {
    decodeTurnReceipt(receipt);
    switch (receipt.kind) {
      case 'partyMoved':
      case 'partyTurned':
      case 'partyAttacked':
        if (!partyIds.has(receipt.actorEntityId)) {
          throw new Error('turn receipt disagrees with party identity');
        }
        break;
      case 'oppositionMoved':
      case 'oppositionPassed':
        if (partyIds.has(receipt.actorEntityId)) {
          throw new Error('turn receipt disagrees with party identity');
        }
        break;
      case 'oppositionAttacked':
        if (
          partyIds.has(receipt.actorEntityId) ||
          !partyIds.has(receipt.target.selectedMemberEntityId)
        ) {
          throw new Error('turn receipt disagrees with party identity');
        }
        break;
      case 'loadoutMoved':
        if (
          !loadoutItemIds.has(receipt.itemEntityId) ||
          (!partyIds.has(receipt.fromOwnerEntityId) &&
            stashOwnerId !== receipt.fromOwnerEntityId) ||
          (!partyIds.has(receipt.toOwnerEntityId) &&
            stashOwnerId !== receipt.toOwnerEntityId)
        ) {
          throw new Error('loadout receipt disagrees with projected ownership');
        }
        break;
      case 'expeditionBegan':
        break;
    }
  }
  if (
    !Array.isArray(value['log']) ||
    value['log'].length > SESSION_VIEW_LIMITS.maxLogEntries
  ) {
    throw new Error('session log is not a bounded array');
  }
  let expectedLogId = 1;
  for (const entry of value['log']) {
    requireExactRecord(entry, SESSION_LOG_ENTRY_KEYS, 'session log entry');
    requireSafeInteger(entry['id'], 1, Number.MAX_SAFE_INTEGER, 'log identity');
    requireSafeInteger(
      entry['revision'],
      1,
      Number(value['revision']),
      'log revision',
    );
    if (entry['id'] !== expectedLogId) {
      throw new Error('session log identities are not canonical');
    }
    expectedLogId += 1;
    decodeTurnReceipt(entry['receipt']);
  }
  const latestLogReceipts = (value['log'] as SessionLogEntry[])
    .filter((entry) => entry.revision === value['revision'])
    .map((entry) => entry.receipt);
  if (
    JSON.stringify(latestLogReceipts) !==
    JSON.stringify(value['latestReceipts'])
  ) {
    throw new Error('latest receipts disagree with the durable session log');
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
  requireBoundedText(
    value['title'],
    1,
    ROGUELIKE_LIMITS.maxAuthoredTextBytes,
    'party member title',
  );
  requireSafeInteger(value['level'], 1, 20, 'party member level');
  requireSafeInteger(
    value['experience'],
    0,
    1_000_000_000,
    'party member experience',
  );
  requireId(value['classId'], 'party member class identity');
  requireBoundedText(
    value['className'],
    1,
    ROGUELIKE_LIMITS.maxAuthoredTextBytes,
    'party member class name',
  );
  requireSafeInteger(value['classLevel'], 1, 20, 'party member class level');
  if (Number(value['classLevel']) > Number(value['level'])) {
    throw new Error('party class level exceeds character level');
  }
  requireSafeInteger(value['currentVitality'], 0, 65_535, 'current vitality');
  requireSafeInteger(value['maximumVitality'], 1, 65_535, 'maximum vitality');
  if (Number(value['currentVitality']) > Number(value['maximumVitality'])) {
    throw new Error('party vitality exceeds its maximum');
  }
  if (value['conscious'] !== Number(value['currentVitality']) > 0) {
    throw new Error('party consciousness disagrees with vitality');
  }
  decodeBoundedUniqueReadouts(value['abilities'], 'abilities', (ability) => {
    requireExactRecord(ability, ABILITY_KEYS, 'ability readout');
    requireId(ability['abilityId'], 'ability identity');
    requireSafeInteger(ability['score'], 1, 30, 'ability score');
    requireI16(ability['modifier'], 'ability modifier');
    if (
      ability['modifier'] !== Math.floor((Number(ability['score']) - 10) / 2)
    ) {
      throw new Error('ability modifier disagrees with its score');
    }
    return ability['abilityId'];
  });
  decodeBoundedUniqueReadouts(value['defenses'], 'defenses', (defense) => {
    requireExactRecord(defense, DEFENSE_KEYS, 'defense readout');
    requireId(defense['defenseId'], 'defense identity');
    requireI16(defense['value'], 'defense value');
    return defense['defenseId'];
  });
  decodeBoundedUniqueReadouts(value['feats'], 'feats', (feat) => {
    requireExactRecord(feat, FEAT_KEYS, 'feat readout');
    requireId(feat['featId'], 'feat identity');
    requireBoundedText(
      feat['name'],
      1,
      ROGUELIKE_LIMITS.maxAuthoredTextBytes,
      'feat name',
    );
    requireBoundedText(
      feat['description'],
      1,
      ROGUELIKE_LIMITS.maxAuthoredTextBytes,
      'feat description',
    );
    return feat['featId'];
  });
  decodeBoundedUniqueReadouts(
    value['actions'],
    'character actions',
    (action) => {
      requireExactRecord(action, CHARACTER_ACTION_KEYS, 'character action');
      requireId(action['actionId'], 'character action identity');
      requireBoundedText(
        action['name'],
        1,
        ROGUELIKE_LIMITS.maxAuthoredTextBytes,
        'character action name',
      );
      return action['actionId'];
    },
  );
  decodeLoadout(value['loadout'], true);
}

function decodeBoundedUniqueReadouts(
  value: unknown,
  label: string,
  decode: (entry: unknown) => string,
): void {
  if (
    !Array.isArray(value) ||
    value.length > ROGUELIKE_LIMITS.maxDefinitionsPerKind
  ) {
    throw new Error(`${label} are not a bounded array`);
  }
  const identities = new Set<string>();
  for (const entry of value) {
    const identity = decode(entry);
    if (identities.has(identity)) {
      throw new Error(`${label} contain duplicate identities`);
    }
    identities.add(identity);
  }
}

function decodeLoadout(
  value: unknown,
  equipmentRequired: boolean,
): asserts value is LoadoutView {
  requireExactRecord(value, LOADOUT_KEYS, 'loadout');
  requireEntityId(value['ownerEntityId'], 'loadout owner identity');
  requireExactRecord(
    value['capacity'],
    LOADOUT_CAPACITY_KEYS,
    'loadout capacity',
  );
  requireSafeInteger(
    value['capacity']['maximum'],
    1,
    ROGUELIKE_LIMITS.maxDefinitionsPerKind,
    'loadout maximum',
  );
  requireSafeInteger(
    value['capacity']['used'],
    0,
    Number(value['capacity']['maximum']),
    'loadout usage',
  );
  if (
    !Array.isArray(value['inventorySlots']) ||
    value['inventorySlots'].length !== value['capacity']['maximum']
  ) {
    throw new Error('inventory slots disagree with loadout capacity');
  }
  const items = new Map<number, LoadoutItemView>();
  for (const item of value['inventorySlots']) {
    if (item === null) {
      continue;
    }
    decodeLoadoutItem(item);
    if (items.has(item.entityId)) {
      throw new Error('loadout contains duplicate item entities');
    }
    items.set(item.entityId, item);
  }
  if (items.size !== value['capacity']['used']) {
    throw new Error('loadout usage disagrees with inventory slots');
  }
  if (!Array.isArray(value['equipmentSlots'])) {
    throw new Error('equipment slots are not an array');
  }
  if (
    (equipmentRequired && value['equipmentSlots'].length !== 3) ||
    (!equipmentRequired && value['equipmentSlots'].length !== 0)
  ) {
    throw new Error('loadout has the wrong equipment slot set');
  }
  const slots = new Set<string>();
  for (const slot of value['equipmentSlots']) {
    requireExactRecord(slot, EQUIPMENT_SLOT_KEYS, 'equipment slot');
    requireId(slot['slotId'], 'equipment slot identity');
    requireBoundedText(
      slot['label'],
      1,
      ROGUELIKE_LIMITS.maxAuthoredTextBytes,
      'equipment slot label',
    );
    if (slots.has(slot['slotId'])) {
      throw new Error('loadout contains duplicate equipment slots');
    }
    slots.add(slot['slotId']);
    if (slot['equipped'] !== null) {
      decodeLoadoutItem(slot['equipped']);
      const inventory = items.get(slot['equipped'].entityId);
      if (
        inventory === undefined ||
        inventory.itemId !== slot['equipped'].itemId ||
        slot['equipped'].equipmentSlotId !== slot['slotId'] ||
        slot['equipped'].equippedSlotId !== slot['slotId'] ||
        inventory.equippedSlotId !== slot['slotId']
      ) {
        throw new Error('equipment assignment disagrees with inventory facts');
      }
    }
  }
  if (
    [...items.values()].some(
      (item) => item.equippedSlotId !== null && !slots.has(item.equippedSlotId),
    )
  ) {
    throw new Error('inventory item references an unknown equipment slot');
  }
}

function decodeLoadoutItem(value: unknown): asserts value is LoadoutItemView {
  requireExactRecord(value, LOADOUT_ITEM_KEYS, 'loadout item');
  requireEntityId(value['entityId'], 'loadout item entity identity');
  requireId(value['itemId'], 'loadout item identity');
  requireBoundedText(
    value['name'],
    1,
    ROGUELIKE_LIMITS.maxAuthoredTextBytes,
    'loadout item name',
  );
  for (const [field, label] of [
    ['equipmentSlotId', 'equipment slot'],
    ['equippedSlotId', 'equipped slot'],
  ] as const) {
    if (value[field] !== null) {
      requireId(value[field], label);
    }
  }
  if (
    value['equippedSlotId'] !== null &&
    value['equippedSlotId'] !== value['equipmentSlotId']
  ) {
    throw new Error('loadout item is equipped in an incompatible slot');
  }
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
    case 'loadoutMoved':
      requireExactRecord(
        value,
        LOADOUT_MOVED_RECEIPT_KEYS,
        'loadout movement receipt',
      );
      requireEntityId(value['itemEntityId'], 'loadout receipt item identity');
      requireEntityId(
        value['fromOwnerEntityId'],
        'loadout receipt source owner identity',
      );
      requireEntityId(
        value['toOwnerEntityId'],
        'loadout receipt destination owner identity',
      );
      if (value['destinationSlotId'] !== null) {
        requireId(value['destinationSlotId'], 'loadout receipt slot identity');
      }
      return;
    case 'expeditionBegan':
      requireExactRecord(
        value,
        EXPEDITION_BEGAN_RECEIPT_KEYS,
        'expedition start receipt',
      );
      return;
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
