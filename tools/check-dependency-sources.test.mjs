import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

import { validateDependencySources } from './check-dependency-sources.mjs';

const inputs = {
  sources: JSON.parse(await readFile('dependency-sources.json', 'utf8')),
  packageJson: JSON.parse(await readFile('package.json', 'utf8')),
  cargoManifest: await readFile('rust/Cargo.toml', 'utf8'),
  cargoLock: await readFile('rust/Cargo.lock', 'utf8'),
  pnpmLock: await readFile('pnpm-lock.yaml', 'utf8'),
  pnpmWorkspace: await readFile('pnpm-workspace.yaml', 'utf8'),
  agentGuidance: await readFile('AGENTS.md', 'utf8'),
  design: await readFile('docs/design.md', 'utf8'),
  sourceProvenance: await readFile('docs/source-provenance.md', 'utf8'),
};

test('accepts the adjacent Engine facade and exact Procgen identity', () => {
  assert.doesNotThrow(() => validateDependencySources(inputs));
});

test('rejects reintroduction of an Engine renderer package', () => {
  const packageJson = structuredClone(inputs.packageJson);
  packageJson.dependencies['@rusty-engine/renderer-host'] = 'forbidden';
  assert.throws(
    () => validateDependencySources({ ...inputs, packageJson }),
    /package.json contains a forbidden Engine TypeScript package/,
  );
});

test('rejects a stale Engine renderer workspace allowlist entry', () => {
  const pnpmWorkspace = `${inputs.pnpmWorkspace}\n  '@rusty-engine/renderer-three': true\n`;
  assert.throws(
    () => validateDependencySources({ ...inputs, pnpmWorkspace }),
    /pnpm-workspace.yaml contains a forbidden Engine TypeScript package/,
  );
});

test('rejects removal of the Procgen package record', () => {
  const mutatedLock = inputs.cargoLock.replace(
    /\[\[package\]\]\nname = "rusty-procgen-preflight"[\s\S]*?(?=\n\[\[package\]\])/,
    '',
  );
  assert.throws(
    () => validateDependencySources({ ...inputs, cargoLock: mutatedLock }),
    /rusty-procgen-preflight locked package record expected 1, observed 0/,
  );
});

test('rejects removal of the Engine facade package record', () => {
  const facadeRecord =
    /\[\[package\]\]\nname = "rusty-engine"[\s\S]*?(?=\n\[\[package\]\])/;
  const mutatedLock = inputs.cargoLock.replace(facadeRecord, '');
  assert.throws(
    () => validateDependencySources({ ...inputs, cargoLock: mutatedLock }),
    /rusty-engine locked package record expected 1, observed 0/,
  );
});

test('rejects a selective direct Engine crate', () => {
  const cargoManifest = `${inputs.cargoManifest}\ncore-ids = { path = "../../rusty-engine/rust/crates/core-ids" }\n`;
  assert.throws(
    () => validateDependencySources({ ...inputs, cargoManifest }),
    /core-ids must be reached through the rusty-engine facade/,
  );
});

test('rejects stale Engine pin language in current architecture docs', () => {
  const design = `${inputs.design}\n\nSeeded rolls use the pinned Engine RNG service.\n`;
  assert.throws(
    () => validateDependencySources({ ...inputs, design }),
    /docs\/design\.md contains stale Engine pin or revision-carrier guidance/,
  );
});

test('rejects Engine-first pin and revision carrier claims', () => {
  for (const claim of [
    'The Engine RNG service is pinned to revision 0123456789012345678901234567890123456789.',
    'Use Engine revision 0123456789012345678901234567890123456789 as the current source identity.',
  ]) {
    const design = `${inputs.design}\n\n${claim}\n`;
    assert.throws(
      () => validateDependencySources({ ...inputs, design }),
      /docs\/design\.md contains stale Engine pin or revision-carrier guidance/,
    );
  }
});

test('does not let a historical sentence exempt a current pin assertion', () => {
  const sourceProvenance = `${inputs.sourceProvenance}\n\nHistorical migration provenance used Engine revision 0123456789012345678901234567890123456789. The current Engine RNG service is pinned to revision 1111111111111111111111111111111111111111.\n`;
  assert.throws(
    () => validateDependencySources({ ...inputs, sourceProvenance }),
    /docs\/source-provenance\.md contains stale Engine pin or revision-carrier guidance/,
  );
});

test('does not let a historical clause exempt a current pin assertion', () => {
  for (const separator of [';', ', but']) {
    const sourceProvenance = `${inputs.sourceProvenance}\n\nHistorical migration provenance used Engine revision abcdef1${separator} the current Engine RNG service is pinned to revision 1111111111111111111111111111111111111111.\n`;
    assert.throws(
      () => validateDependencySources({ ...inputs, sourceProvenance }),
      /docs\/source-provenance\.md contains stale Engine pin or revision-carrier guidance/,
    );
  }
});

test('rejects current Engine resolution wording', () => {
  for (const claim of [
    'Engine currently resolves to revision 1111111111111111111111111111111111111111 for every build.',
    'The current runtime resolves Engine to revision 1111111111111111111111111111111111111111 for every build.',
    'The current runtime resolves revision 1111111111111111111111111111111111111111 for Engine on every build.',
    'Revision 1111111111111111111111111111111111111111 is the current Engine source for every build.',
    'Production ships revision 1111111111111111111111111111111111111111 for Engine.',
    'Production ships commit 1111111111111111111111111111111111111111 for Engine.',
    'Production ships Engine from SHA 1111111111111111111111111111111111111111.',
  ]) {
    const design = `${inputs.design}\n\n${claim}\n`;
    assert.throws(
      () => validateDependencySources({ ...inputs, design }),
      /docs\/design\.md contains stale Engine pin or revision-carrier guidance/,
    );
  }
});

test('does not let unrelated negation exempt a current carrier clause', () => {
  const design = `${inputs.design}\n\nEngine revision identity is not absent; use Engine revision 1111111111111111111111111111111111111111 as the current source.\n`;
  assert.throws(
    () => validateDependencySources({ ...inputs, design }),
    /docs\/design\.md contains stale Engine pin or revision-carrier guidance/,
  );
});

test('allows explicitly historical Engine revision provenance', () => {
  const sourceProvenance = `${inputs.sourceProvenance}\n\nHistorical migration note: exact reviewed Engine revision 0123456789012345678901234567890123456789.\n`;
  assert.doesNotThrow(() =>
    validateDependencySources({ ...inputs, sourceProvenance }),
  );
});

test('allows exact Procgen pin language', () => {
  const design = `${inputs.design}\n\nRusty Procgen is pinned to exact revision 722e2c479bdf88ab39b66d2d33ab466b698ec7df.\n`;
  assert.doesNotThrow(() => validateDependencySources({ ...inputs, design }));
});

test('allows the exact adjacent-facade negative assertion', () => {
  const design = `${inputs.design}\n\nEngine revision identity is not a game runtime or persistence fact.\n`;
  assert.doesNotThrow(() => validateDependencySources({ ...inputs, design }));
});
