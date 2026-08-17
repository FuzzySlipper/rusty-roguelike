/**
 * Damage type definitions. Values mirror `rust/content/rules/starter.json`
 * exactly.
 */

import { damageType, type DamageTypeDefinition } from '../authoring/mod.js';

export const damageTypes: readonly DamageTypeDefinition[] = [
  damageType('physical'),
  damageType('fire'),
  damageType('psychic'),
];
