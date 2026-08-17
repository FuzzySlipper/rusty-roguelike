/**
 * Action definitions: everything an actor can attempt. Movement actions move
 * exactly one grid step and target self (fixed policy). Attacks target a
 * hostile cell or the hostile party square. Values match the committed
 * `data/gameplay/rusty-roguelike-starter.package.json` artifact exactly.
 */

import {
  attackAction,
  damage,
  moveAction,
  type ActionDefinition,
} from '../authoring/mod.js';

export const actions: readonly ActionDefinition[] = [
  moveAction('move', 'Move', ['movement']),
  attackAction(
    'guardian-strike',
    'Guardian Strike',
    ['martial', 'melee'],
    'hostile-cell',
    {
      ability: 'might',
      defense: 'armor',
      damage: damage('physical', 1, 8, 2),
      range: 1,
    },
  ),
  attackAction(
    'sweeping-strike',
    'Sweeping Strike',
    ['martial', 'melee'],
    'hostile-cell',
    {
      ability: 'might',
      defense: 'grit',
      damage: damage('physical', 1, 10, 0),
      range: 1,
    },
  ),
  attackAction(
    'aimed-shot',
    'Aimed Shot',
    ['martial', 'ranged'],
    'hostile-cell',
    {
      ability: 'finesse',
      defense: 'armor',
      damage: damage('physical', 1, 8, 1),
      range: 6,
    },
  ),
  attackAction(
    'quick-shot',
    'Quick Shot',
    ['martial', 'ranged'],
    'hostile-cell',
    {
      ability: 'finesse',
      defense: 'wits',
      damage: damage('physical', 1, 6, 0),
      range: 4,
    },
  ),
  attackAction(
    'arcane-bolt',
    'Arcane Bolt',
    ['arcane', 'ranged'],
    'hostile-cell',
    {
      ability: 'intellect',
      defense: 'wits',
      damage: damage('psychic', 1, 8, 1),
      range: 5,
    },
  ),
  attackAction(
    'flame-burst',
    'Flame Burst',
    ['arcane', 'fire'],
    'hostile-cell',
    {
      ability: 'intellect',
      defense: 'grit',
      damage: damage('fire', 2, 6, 0),
      range: 4,
    },
  ),
  attackAction(
    'mind-spike',
    'Mind Spike',
    ['arcane', 'psychic'],
    'hostile-cell',
    {
      ability: 'spirit',
      defense: 'nerve',
      damage: damage('psychic', 1, 10, 0),
      range: 3,
    },
  ),
  attackAction(
    'rusty-blade',
    'Rusty Blade',
    ['monster', 'melee'],
    'hostile-party-square',
    {
      ability: 'finesse',
      defense: 'armor',
      damage: damage('physical', 1, 6, 0),
      range: 1,
    },
  ),
  attackAction(
    'ember-shot',
    'Ember Shot',
    ['monster', 'fire', 'ranged'],
    'hostile-party-square',
    {
      ability: 'spirit',
      defense: 'nerve',
      damage: damage('fire', 1, 6, 1),
      range: 4,
    },
  ),
];
