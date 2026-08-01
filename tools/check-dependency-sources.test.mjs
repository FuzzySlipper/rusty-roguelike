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

test('rejects removal of one Engine package record', () => {
  const dependency = '@rusty-engine/renderer-host';
  const lockedBase = `https://codeload.github.com/FuzzySlipper/rusty-engine/tar.gz/${inputs.sources.rustyEngine.commit}#path:render/packages/renderer-host`;
  const mutatedLock = removeYamlRecord(
    inputs.pnpmLock,
    `  '${dependency}@${lockedBase}':\n`,
  );
  assert.throws(
    () => validateDependencySources({ ...inputs, pnpmLock: mutatedLock }),
    /renderer-host locked package record expected 1, observed 0/,
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

test('rejects removal of one Engine Rust crate package record', () => {
  const mutatedLock = inputs.cargoLock.replace(
    /\[\[package\]\]\nname = "gameplay-rules"[\s\S]*?(?=\n\[\[package\]\])/,
    '',
  );
  assert.throws(
    () => validateDependencySources({ ...inputs, cargoLock: mutatedLock }),
    /gameplay-rules locked package record expected 1, observed 0/,
  );
});

function removeYamlRecord(content, header) {
  const start = content.indexOf(header);
  assert.notEqual(start, -1, `missing fixture record ${header}`);
  const next = content.indexOf("\n  '", start + header.length);
  assert.notEqual(next, -1, `fixture record ${header} has no successor`);
  return `${content.slice(0, start)}${content.slice(next + 1)}`;
}
