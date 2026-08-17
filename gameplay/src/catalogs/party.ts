/**
 * The collapsed-party definition. Members must equal exactly the set of
 * party-side actors. Values match the committed `data/gameplay/rusty-roguelike-starter.package.json` artifact
 * exactly.
 */

import { party, type PartyDefinition } from '../authoring/mod.js';

export const partyDefinition: PartyDefinition = party('lantern-company', 100, [
  'brann',
  'kestrel',
  'mira',
]);
