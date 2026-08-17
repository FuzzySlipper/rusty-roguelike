/**
 * Package envelope composition for Rusty Roguelike. Provenance is computed
 * here from the section -> source-file map the package entry supplies, so no
 * catalog file ever hand-writes subject/source pairs that rot on the next
 * edit.
 *
 * Subjects follow the exact kinds the Rust compiler correlates
 * (compiler.rs `definition_origin`): `ability:<id>`, `defense:<id>`,
 * `damage-type:<id>`, `action:<id>`, `feat:<id>`, `class:<id>`,
 * `item:<id>`, `actor:<id>`, `party:<id>`. Note the damage-type kind is
 * `damage-type`, not the `damageTypes` payload section name.
 */

import type {
  AbilityDefinition,
  ActionDefinition,
  ActorDefinition,
  ClassDefinition,
  DamageTypeDefinition,
  DefenseDefinition,
  FeatDefinition,
  ItemDefinition,
  PartyDefinition,
  RollPolicyDefinition,
} from './definitions.js';

export type RoguelikeGameplayPayload = Readonly<{
  schemaVersion: 1;
  rollPolicy: RollPolicyDefinition;
  abilities: readonly AbilityDefinition[];
  defenses: readonly DefenseDefinition[];
  damageTypes: readonly DamageTypeDefinition[];
  actions: readonly ActionDefinition[];
  feats: readonly FeatDefinition[];
  classes: readonly ClassDefinition[];
  items: readonly ItemDefinition[];
  actors: readonly ActorDefinition[];
  party: PartyDefinition;
}>;

export type PackageInput = Readonly<{
  /** Package id inside the `rusty-roguelike` domain, e.g. "starter". */
  packageId: string;
  version: number;
  /** Section name -> source path relative to the repository root. */
  sources: Readonly<Record<string, string>>;
  payload: RoguelikeGameplayPayload;
}>;

export const composePackage = (input: PackageInput) => {
  const { payload } = input;
  const sources = Object.entries(input.sources).map(([entryId, path]) => ({
    id: entryId,
    path,
  }));
  const provenance: { subject: string; source: string }[] = [];
  const record = (source: string, subjects: readonly string[]): void => {
    for (const subject of subjects) {
      provenance.push({ subject, source });
    }
  };
  record(
    'abilities',
    payload.abilities.map((entry) => `ability:${entry.id}`),
  );
  record(
    'defenses',
    payload.defenses.map((entry) => `defense:${entry.id}`),
  );
  record(
    'damageTypes',
    payload.damageTypes.map((entry) => `damage-type:${entry.id}`),
  );
  record(
    'actions',
    payload.actions.map((entry) => `action:${entry.id}`),
  );
  record(
    'feats',
    payload.feats.map((entry) => `feat:${entry.id}`),
  );
  record(
    'classes',
    payload.classes.map((entry) => `class:${entry.id}`),
  );
  record(
    'items',
    payload.items.map((entry) => `item:${entry.id}`),
  );
  record(
    'actors',
    payload.actors.map((entry) => `actor:${entry.id}`),
  );
  record('party', [`party:${payload.party.id}`]);
  return {
    kind: 'rusty.gameplay-rules.package' as const,
    schemaVersion: 1 as const,
    domain: 'rusty-roguelike' as const,
    package: input.packageId,
    version: input.version,
    dependencies: [] as const,
    sources,
    provenance,
    payload,
  };
};
