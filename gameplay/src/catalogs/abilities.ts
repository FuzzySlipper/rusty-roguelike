/**
 * Ability definitions: the four core ability scores with their legal ranges.
 * Values match the committed `data/gameplay/rusty-roguelike-starter.package.json` artifact exactly.
 */

import { ability, type AbilityDefinition } from '../authoring/mod.js';

export const abilities: readonly AbilityDefinition[] = [
  ability('might', 1, 30),
  ability('finesse', 1, 30),
  ability('intellect', 1, 30),
  ability('spirit', 1, 30),
];
