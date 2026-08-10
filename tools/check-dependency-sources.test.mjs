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
