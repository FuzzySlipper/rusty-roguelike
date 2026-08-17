/**
 * Feat definitions. Values mirror `rust/content/rules/starter.json` exactly.
 */

import { feat, statModifier, type FeatDefinition } from '../authoring/mod.js';

export const feats: readonly FeatDefinition[] = [
  feat(
    'hold-the-line',
    'Hold the Line',
    'A disciplined stance that hardens the front rank.',
    [statModifier('armor', 1)],
  ),
  feat(
    'defensive-mobility',
    'Defensive Mobility',
    'Footwork makes the scout harder to pin down.',
    [statModifier('armor', 1)],
  ),
  feat(
    'arcane-focus',
    'Arcane Focus',
    'Practice reinforces concentration under pressure.',
    [statModifier('wits', 1)],
  ),
  feat(
    'goblin-cunning',
    'Goblin Cunning',
    'Instinct favors a quick retreat from danger.',
    [statModifier('wits', 1)],
  ),
  feat(
    'burning-gaze',
    'Burning Gaze',
    'A supernatural glare steadies the ember eye.',
    [statModifier('nerve', 1)],
  ),
];
