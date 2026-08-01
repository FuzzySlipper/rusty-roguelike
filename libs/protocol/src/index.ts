export * from './generated/api-types';

import {
  BOOTSTRAP_SCHEMA_VERSION,
  RUSTY_ENGINE_REVISION,
  RUSTY_PROCGEN_REVISION,
  type BootstrapReadoutDto,
} from './generated/api-types';

const BOOTSTRAP_KEYS = [
  'phase',
  'procgenLinkHash',
  'product',
  'rustyEngineRevision',
  'rustyProcgenRevision',
  'schemaVersion',
] as const;

export function decodeBootstrapReadout(value: unknown): BootstrapReadoutDto {
  if (!isRecord(value)) {
    throw new Error('bootstrap response must be an object');
  }
  const keys = Object.keys(value).sort();
  if (
    keys.length !== BOOTSTRAP_KEYS.length ||
    keys.some((key, index) => key !== BOOTSTRAP_KEYS[index])
  ) {
    throw new Error('bootstrap response contains missing or unknown fields');
  }
  if (value['schemaVersion'] !== BOOTSTRAP_SCHEMA_VERSION) {
    throw new Error('bootstrap response has an unsupported schema');
  }
  if (
    value['product'] !== 'rusty-roguelike' ||
    value['phase'] !== 'bootstrap'
  ) {
    throw new Error('bootstrap response has the wrong product identity');
  }
  if (value['rustyEngineRevision'] !== RUSTY_ENGINE_REVISION) {
    throw new Error('bootstrap response has the wrong Rusty Engine revision');
  }
  if (value['rustyProcgenRevision'] !== RUSTY_PROCGEN_REVISION) {
    throw new Error('bootstrap response has the wrong Rusty Procgen revision');
  }
  if (
    typeof value['procgenLinkHash'] !== 'string' ||
    !/^fnv1a64:[0-9a-f]{16}$/.test(value['procgenLinkHash'])
  ) {
    throw new Error('bootstrap response has an invalid Procgen linkage hash');
  }
  return value as BootstrapReadoutDto;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
