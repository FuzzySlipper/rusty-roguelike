/**
 * The starter gameplay package for Rusty Roguelike: the retained starter
 * content composed into the deterministic gameplay-rules envelope Rust
 * admits. Materialization walks `packages/`; one entry per package.
 */

import { composePackage, seededRollPolicy } from '../authoring/mod.js';
import { abilities } from '../catalogs/abilities.js';
import { actions } from '../catalogs/actions.js';
import { actors } from '../catalogs/actors.js';
import { classes } from '../catalogs/classes.js';
import { damageTypes } from '../catalogs/damageTypes.js';
import { defenses } from '../catalogs/defenses.js';
import { feats } from '../catalogs/feats.js';
import { items } from '../catalogs/items.js';
import { partyDefinition } from '../catalogs/party.js';

export const gameplayPackage = composePackage({
  packageId: 'starter',
  version: 1,
  sources: {
    abilities: 'gameplay/src/catalogs/abilities.ts',
    defenses: 'gameplay/src/catalogs/defenses.ts',
    damageTypes: 'gameplay/src/catalogs/damageTypes.ts',
    actions: 'gameplay/src/catalogs/actions.ts',
    feats: 'gameplay/src/catalogs/feats.ts',
    classes: 'gameplay/src/catalogs/classes.ts',
    items: 'gameplay/src/catalogs/items.ts',
    actors: 'gameplay/src/catalogs/actors.ts',
    party: 'gameplay/src/catalogs/party.ts',
  },
  payload: {
    schemaVersion: 1,
    rollPolicy: seededRollPolicy(424242),
    abilities,
    defenses,
    damageTypes,
    actions,
    feats,
    classes,
    items,
    actors,
    party: partyDefinition,
  },
});
