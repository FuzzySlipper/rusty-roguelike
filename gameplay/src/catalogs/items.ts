/**
 * Item definitions: equipment slots, granted actions, and defense
 * modifiers. Values match the committed `data/gameplay/rusty-roguelike-starter.package.json` artifact exactly.
 */

import { item, statModifier, type ItemDefinition } from '../authoring/mod.js';

export const items: readonly ItemDefinition[] = [
  item('longsword', 'Longsword', {
    slot: 'weapon',
    grantsAction: 'sweeping-strike',
  }),
  item('scale-mail', 'Scale Mail', {
    slot: 'body',
    modifiers: [statModifier('armor', 2)],
  }),
  item('shortbow', 'Shortbow', { slot: 'weapon', grantsAction: 'quick-shot' }),
  item('leather-armor', 'Leather Armor', {
    slot: 'body',
    modifiers: [statModifier('armor', 1)],
  }),
  item('ash-staff', 'Ash Staff', { slot: 'weapon' }),
  item('focus-orb', 'Focus Orb', {
    slot: 'focus',
    grantsAction: 'mind-spike',
    modifiers: [statModifier('wits', 1)],
  }),
  item('traveling-robes', 'Traveling Robes', { slot: 'body' }),
  item('rusty-knife', 'Rusty Knife', { slot: 'weapon' }),
  item('ember-focus', 'Ember Focus', {
    slot: 'focus',
    modifiers: [statModifier('nerve', 1)],
  }),
];
