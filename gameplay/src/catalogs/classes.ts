/**
 * Class definitions with their level-1 action/feat grants and slot
 * increases. Values match the committed `data/gameplay/rusty-roguelike-starter.package.json` artifact exactly.
 */

import {
  classDefinition,
  classLevel,
  type ClassDefinition,
} from '../authoring/mod.js';

export const classes: readonly ClassDefinition[] = [
  classDefinition('guardian', 'Guardian', [
    classLevel(1, ['move', 'guardian-strike'], ['hold-the-line'], 3, 1),
  ]),
  classDefinition('scout', 'Scout', [
    classLevel(1, ['move', 'aimed-shot'], ['defensive-mobility'], 3, 1),
  ]),
  classDefinition('arcanist', 'Arcanist', [
    classLevel(
      1,
      ['move', 'arcane-bolt', 'flame-burst'],
      ['arcane-focus'],
      4,
      1,
    ),
  ]),
  classDefinition('goblin-raider', 'Goblin Raider', [
    classLevel(1, ['move', 'rusty-blade'], ['goblin-cunning'], 2, 1),
  ]),
  classDefinition('ember-eye', 'Ember Eye', [
    classLevel(1, ['move', 'ember-shot'], ['burning-gaze'], 2, 1),
  ]),
];
