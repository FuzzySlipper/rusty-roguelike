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
};

test('accepts every canonical exact dependency identity', () => {
  assert.doesNotThrow(() => validateDependencySources(inputs));
});

test('rejects removal of one declared legacy Engine renderer package', () => {
  const packageJson = structuredClone(inputs.packageJson);
  delete packageJson.dependencies['@rusty-engine/renderer-host'];
  assert.throws(
    () => validateDependencySources({ ...inputs, packageJson }),
    /renderer-host is not bound to the declared legacy renderer revision/,
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
  const cargoManifest = `${inputs.cargoManifest}\ncore-ids = { git = "${inputs.sources.rustyEngine.repository}", branch = "main" }\n`;
  assert.throws(
    () => validateDependencySources({ ...inputs, cargoManifest }),
    /core-ids must be reached through the rusty-engine facade/,
  );
});
