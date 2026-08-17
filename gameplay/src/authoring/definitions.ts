/**
 * Roguelike authoring grammar: definition shapes and light builders for the
 * Rusty Roguelike starter content. These are data tables; the meaning of
 * every field is owned by the Rust compiler in
 * `rust/crates/rusty-roguelike/src/rules/` (candidate.rs shapes, compiler.rs
 * semantics). TypeScript here is build-time authoring only — it never
 * evaluates gameplay.
 *
 * Gameplay policy is fixed here, not authored:
 * - A movement action must move exactly one grid step and target self only.
 * - The `ally-cell` target is rejected by the compiler for attacks, so it is
 *   not part of this grammar.
 * - Activation cost is not authored; the compiler hardcodes it to 1.
 */

/** Identity contract mirrors `RoguelikeId`: ^[a-z0-9._-]+, at most 64 bytes. */
export type RoguelikeId = string;

const ROGUELIKE_ID_PATTERN = /^[a-z0-9._-]+$/;
const MAX_ROGUELIKE_ID_BYTES = 64;

/** Validates and normalizes an id under the Rust `RoguelikeId` contract. */
export const id = (value: string): RoguelikeId => {
  if (value.length === 0 || value.length > MAX_ROGUELIKE_ID_BYTES) {
    throw new Error(
      `roguelike id must be 1..64 bytes, got ${JSON.stringify(value)}`,
    );
  }
  if (!ROGUELIKE_ID_PATTERN.test(value)) {
    throw new Error(
      `roguelike id ${JSON.stringify(value)} must match ^[a-z0-9._-]+$`,
    );
  }
  return value;
};

export type StaticRollDefinition = Readonly<{
  d20: number;
  damage: readonly number[];
}>;

/**
 * Roll policy mirrors `RollPolicyCandidate`: seeded carries a seed with an
 * empty roll list; static carries bounded caller-supplied rolls and no seed.
 */
export type RollPolicyDefinition =
  | Readonly<{ kind: 'seeded'; seed: number; rolls: readonly [] }>
  | Readonly<{ kind: 'static'; rolls: readonly StaticRollDefinition[] }>;

export const seededRollPolicy = (seed: number): RollPolicyDefinition => {
  if (!Number.isSafeInteger(seed) || seed < 0) {
    throw new Error(
      `seeded roll policy seed must be a non-negative safe integer, got ${seed}`,
    );
  }
  return { kind: 'seeded', seed, rolls: [] };
};

export const staticRollPolicy = (
  rolls: readonly StaticRollDefinition[],
): RollPolicyDefinition => {
  if (rolls.length < 1 || rolls.length > 4096) {
    throw new Error(
      `static roll policy needs 1..4096 rolls, got ${rolls.length}`,
    );
  }
  return { kind: 'static', rolls };
};

export type AbilityDefinition = Readonly<{
  id: RoguelikeId;
  minimum: number;
  maximum: number;
}>;

export const ability = (
  value: RoguelikeId,
  minimum: number,
  maximum: number,
): AbilityDefinition => {
  if (minimum < 1 || maximum > 30 || minimum > maximum) {
    throw new Error(
      `ability ${value} range must satisfy 1 <= minimum <= maximum <= 30`,
    );
  }
  return { id: id(value), minimum, maximum };
};

export type DefenseDefinition = Readonly<{
  id: RoguelikeId;
  base: number;
  abilities: readonly RoguelikeId[];
}>;

export const defense = (
  value: RoguelikeId,
  base: number,
  abilities: readonly RoguelikeId[],
): DefenseDefinition => {
  if (base < -20 || base > 40) {
    throw new Error(`defense ${value} base must be inside -20..=40`);
  }
  return { id: id(value), base, abilities: abilities.map(id) };
};

export type DamageTypeDefinition = Readonly<{
  id: RoguelikeId;
}>;

export const damageType = (value: RoguelikeId): DamageTypeDefinition => ({
  id: id(value),
});

export type DamageDefinition = Readonly<{
  kind: RoguelikeId;
  dice: number;
  sides: number;
  bonus: number;
}>;

export const damage = (
  kind: RoguelikeId,
  dice: number,
  sides: number,
  bonus: number,
): DamageDefinition => {
  if (
    dice < 1 ||
    dice > 16 ||
    sides < 2 ||
    sides > 100 ||
    bonus < -100 ||
    bonus > 100
  ) {
    throw new Error(
      `damage must satisfy dice 1..=16, sides 2..=100, bonus -100..=100, got ${dice}d${sides}+${bonus}`,
    );
  }
  return { kind: id(kind), dice, sides, bonus };
};

/**
 * The two compiler-accepted attack targets. `ally-cell` exists in the
 * candidate enum but is rejected for attacks, so it is deliberately absent
 * here.
 */
export type HostileTarget = 'hostile-cell' | 'hostile-party-square';

/** Movement policy is fixed: exactly one grid step, self only. */
export type MovementDefinition = Readonly<{
  steps: 1;
}>;

export type AttackDefinition = Readonly<{
  ability: RoguelikeId;
  defense: RoguelikeId;
  damage: DamageDefinition;
  range: number;
}>;

export type ActionDefinition = Readonly<{
  id: RoguelikeId;
  name: string;
  tags: readonly RoguelikeId[];
  target: 'self-only' | HostileTarget;
  movement: MovementDefinition | null;
  attack: AttackDefinition | null;
}>;

const MAX_ACTION_TAGS = 16;

const actionBase = (
  value: RoguelikeId,
  name: string,
  tags: readonly RoguelikeId[],
  target: 'self-only' | HostileTarget,
): ActionDefinition => {
  if (tags.length > MAX_ACTION_TAGS) {
    throw new Error(
      `action ${value} may carry at most ${MAX_ACTION_TAGS} tags`,
    );
  }
  if (new Set(tags).size !== tags.length) {
    throw new Error(`action ${value} tags must be distinct`);
  }
  return {
    id: id(value),
    name,
    tags: tags.map(id),
    target,
    movement: null,
    attack: null,
  };
};

/**
 * A movement action: the compiler requires movement to move exactly one grid
 * step and target self only, so both are fixed here.
 */
export const moveAction = (
  value: RoguelikeId,
  name: string,
  tags: readonly RoguelikeId[] = ['movement'],
): ActionDefinition => ({
  ...actionBase(value, name, tags, 'self-only'),
  movement: { steps: 1 },
});

/** An attack action against a hostile cell or the hostile party square. */
export const attackAction = (
  value: RoguelikeId,
  name: string,
  tags: readonly RoguelikeId[],
  target: HostileTarget,
  attack: AttackDefinition,
): ActionDefinition => {
  if (attack.range < 1 || attack.range > 16) {
    throw new Error(`attack ${value} range must be inside 1..=16`);
  }
  return { ...actionBase(value, name, tags, target), attack };
};

export type StatModifierDefinition = Readonly<{
  defense: RoguelikeId;
  amount: number;
}>;

export const statModifier = (
  defense: RoguelikeId,
  amount: number,
): StatModifierDefinition => {
  if (amount < -20 || amount > 20) {
    throw new Error(`modifier on ${defense} amount must be inside -20..=20`);
  }
  return { defense: id(defense), amount };
};

export type FeatDefinition = Readonly<{
  id: RoguelikeId;
  name: string;
  description: string;
  modifiers: readonly StatModifierDefinition[];
}>;

export const feat = (
  value: RoguelikeId,
  name: string,
  description: string,
  modifiers: readonly StatModifierDefinition[],
): FeatDefinition => {
  if (
    new Set(modifiers.map((modifier) => modifier.defense)).size !==
    modifiers.length
  ) {
    throw new Error(`feat ${value} modifiers must reference distinct defenses`);
  }
  return { id: id(value), name, description, modifiers };
};

export type ClassLevelDefinition = Readonly<{
  level: number;
  actions: readonly RoguelikeId[];
  feats: readonly RoguelikeId[];
  actionSlotIncrease: number;
  featSlotIncrease: number;
}>;

export const classLevel = (
  level: number,
  actions: readonly RoguelikeId[],
  feats: readonly RoguelikeId[],
  actionSlotIncrease: number,
  featSlotIncrease: number,
): ClassLevelDefinition => {
  if (level < 1 || level > 20) {
    throw new Error(`class level must be inside 1..=20, got ${level}`);
  }
  if (actionSlotIncrease > 8 || featSlotIncrease > 8) {
    throw new Error(`class level ${level} slot increases must be at most 8`);
  }
  return {
    level,
    actions: actions.map(id),
    feats: feats.map(id),
    actionSlotIncrease,
    featSlotIncrease,
  };
};

export type ClassDefinition = Readonly<{
  id: RoguelikeId;
  name: string;
  levels: readonly ClassLevelDefinition[];
}>;

export const classDefinition = (
  value: RoguelikeId,
  name: string,
  levels: readonly ClassLevelDefinition[],
): ClassDefinition => {
  levels.forEach((entry, index) => {
    if (entry.level !== index + 1) {
      throw new Error(`class ${value} levels must be contiguous starting at 1`);
    }
  });
  return { id: id(value), name, levels };
};

export type EquipmentSlot = 'body' | 'weapon' | 'focus';

export type ItemDefinition = Readonly<{
  id: RoguelikeId;
  name: string;
  slot: EquipmentSlot | null;
  grantsAction: RoguelikeId | null;
  modifiers: readonly StatModifierDefinition[];
}>;

export const item = (
  value: RoguelikeId,
  name: string,
  options: Readonly<{
    slot?: EquipmentSlot;
    grantsAction?: RoguelikeId;
    modifiers?: readonly StatModifierDefinition[];
  }> = {},
): ItemDefinition => ({
  id: id(value),
  name,
  slot: options.slot ?? null,
  grantsAction:
    options.grantsAction === undefined ? null : id(options.grantsAction),
  modifiers: options.modifiers ?? [],
});

export type ActorSide = 'party' | 'opposition';

export type AbilityScoreDefinition = Readonly<{
  ability: RoguelikeId;
  score: number;
}>;

export const abilityScore = (
  ability: RoguelikeId,
  score: number,
): AbilityScoreDefinition => ({
  ability: id(ability),
  score,
});

export type ActorDefinition = Readonly<{
  id: RoguelikeId;
  entityId: number;
  name: string;
  title: string;
  side: ActorSide;
  level: number;
  experience: number;
  vitality: number;
  inventoryCapacity: number;
  class: RoguelikeId;
  classLevel: number;
  abilities: readonly AbilityScoreDefinition[];
  actions: readonly RoguelikeId[];
  feats: readonly RoguelikeId[];
  items: readonly RoguelikeId[];
}>;

export const actor = (
  value: RoguelikeId,
  entityId: number,
  name: string,
  title: string,
  side: ActorSide,
  options: Readonly<{
    vitality: number;
    inventoryCapacity: number;
    class: RoguelikeId;
    level?: number;
    experience?: number;
    classLevel?: number;
    abilities: readonly AbilityScoreDefinition[];
    actions: readonly RoguelikeId[];
    feats: readonly RoguelikeId[];
    items: readonly RoguelikeId[];
  }>,
): ActorDefinition => {
  const MAX_SAFE_JSON_INTEGER = 9_007_199_254_740_991;
  if (entityId < 1 || entityId > MAX_SAFE_JSON_INTEGER) {
    throw new Error(`actor ${value} entityId must be a safe JSON integer >= 1`);
  }
  const level = options.level ?? 1;
  const classLevel = options.classLevel ?? level;
  if (classLevel !== level) {
    throw new Error(`actor ${value} classLevel must equal level`);
  }
  if (
    options.experience !== undefined &&
    (options.experience < 0 || options.experience > 1_000_000_000)
  ) {
    throw new Error(`actor ${value} experience must be inside 0..=1000000000`);
  }
  if (options.vitality < 1) {
    throw new Error(`actor ${value} vitality must be at least 1`);
  }
  if (options.inventoryCapacity < 1 || options.inventoryCapacity > 64) {
    throw new Error(`actor ${value} inventoryCapacity must be inside 1..=64`);
  }
  return {
    id: id(value),
    entityId,
    name,
    title,
    side,
    level,
    experience: options.experience ?? 0,
    vitality: options.vitality,
    inventoryCapacity: options.inventoryCapacity,
    class: id(options.class),
    classLevel,
    abilities: options.abilities,
    actions: options.actions.map(id),
    feats: options.feats.map(id),
    items: options.items.map(id),
  };
};

export type PartyDefinition = Readonly<{
  id: RoguelikeId;
  entityId: number;
  members: readonly RoguelikeId[];
}>;

export const party = (
  value: RoguelikeId,
  entityId: number,
  members: readonly RoguelikeId[],
): PartyDefinition => ({
  id: id(value),
  entityId,
  members: members.map(id),
});
