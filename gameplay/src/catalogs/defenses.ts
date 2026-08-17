/**
 * Defense definitions: base value and the ability each defense keys off.
 * Values match the committed `data/gameplay/rusty-roguelike-starter.package.json` artifact exactly.
 */

import { defense, type DefenseDefinition } from '../authoring/mod.js';

export const defenses: readonly DefenseDefinition[] = [
  defense('armor', 8, ['finesse']),
  defense('grit', 8, ['might']),
  defense('wits', 8, ['intellect']),
  defense('nerve', 8, ['spirit']),
];
