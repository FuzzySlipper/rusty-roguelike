import { describe, expect, it } from 'vitest';

import {
  decodeBootstrapReadout,
  RUSTY_ENGINE_REVISION,
  RUSTY_PROCGEN_REVISION,
} from './index';

const VALID = {
  schemaVersion: 1,
  product: 'rusty-roguelike',
  phase: 'bootstrap',
  rustyEngineRevision: RUSTY_ENGINE_REVISION,
  rustyProcgenRevision: RUSTY_PROCGEN_REVISION,
  procgenLinkHash: `fnv1a64:${'a'.repeat(16)}`,
};

describe('decodeBootstrapReadout', () => {
  it('accepts the exact Rust-owned bootstrap shape', () => {
    expect(decodeBootstrapReadout(VALID)).toEqual(VALID);
  });

  it('rejects unknown fields and dependency drift', () => {
    expect(() => decodeBootstrapReadout({ ...VALID, extra: true })).toThrow(
      'missing or unknown',
    );
    expect(() =>
      decodeBootstrapReadout({
        ...VALID,
        rustyProcgenRevision: '0'.repeat(40),
      }),
    ).toThrow('wrong Rusty Procgen');
    expect(() =>
      decodeBootstrapReadout({ ...VALID, procgenLinkHash: 'a'.repeat(64) }),
    ).toThrow('invalid Procgen linkage hash');
  });
});
