/**
 * The collapsed-party definition. Members must equal exactly the set of
 * party-side actors. Values mirror `rust/content/rules/starter.json`
 * exactly.
 */

import { party, type PartyDefinition } from '../authoring/mod.js';

export const partyDefinition: PartyDefinition = party('lantern-company', 100, [
  'brann',
  'kestrel',
  'mira',
]);
