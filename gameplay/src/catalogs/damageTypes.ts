/**
 * Damage type definitions. Values match the committed `data/gameplay/rusty-roguelike-starter.package.json` artifact
 * exactly.
 */

import { damageType, type DamageTypeDefinition } from '../authoring/mod.js';

export const damageTypes: readonly DamageTypeDefinition[] = [
  damageType('physical'),
  damageType('fire'),
  damageType('psychic'),
];
