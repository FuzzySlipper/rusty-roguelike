import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

const ENGINE_PACKAGES = [
  '@rusty-engine/render-contracts',
  '@rusty-engine/render-projection',
  '@rusty-engine/renderer-host',
  '@rusty-engine/renderer-three',
];

const ENGINE_RUST_CRATES = [
  'core-ids',
  'core-space',
  'core-voxel',
  'entity-state',
  'gameplay-mechanics',
  'gameplay-rules',
  'svc-collision',
  'svc-pathfinding',
  'svc-rng',
  'svc-spatial',
  'svc-volume',
];

export function validateDependencySources({
  sources,
  packageJson,
  cargoManifest,
  cargoLock,
  pnpmLock,
}) {
  for (const [name, source] of Object.entries({
    rustyEngine: sources.rustyEngine,
    rustyProcgen: sources.rustyProcgen,
  })) {
    if (
      !/^https:\/\/github\.com\/FuzzySlipper\/[a-z0-9-]+$/.test(
        source.repository,
      )
    ) {
      throw new Error(
        `${name} repository is not a canonical public GitHub source`,
      );
    }
    if (!/^[0-9a-f]{40}$/.test(source.commit)) {
      throw new Error(`${name} commit is not an exact 40-character revision`);
    }
  }

  const packageSections = splitPnpmLock(pnpmLock);
  for (const dependency of ENGINE_PACKAGES) {
    const packagePath = `render/packages/${dependency.split('/')[1]}`;
    const expectedSpecifier = `github:FuzzySlipper/rusty-engine#${sources.rustyEngine.commit}&path:${packagePath}`;
    const lockedBase = `https://codeload.github.com/FuzzySlipper/rusty-engine/tar.gz/${sources.rustyEngine.commit}#path:${packagePath}`;
    if (packageJson.dependencies?.[dependency] !== expectedSpecifier) {
      throw new Error(
        `${dependency} does not use the canonical Engine revision`,
      );
    }
    requireCount(
      packageSections.importer,
      `specifier: ${expectedSpecifier}`,
      1,
      `${dependency} importer specifier`,
    );
    requireLinePrefixCount(
      packageSections.importer,
      `version: ${lockedBase}`,
      1,
      `${dependency} importer resolution`,
    );
    requireLinePrefixCount(
      packageSections.packages,
      `'${dependency}@${lockedBase}`,
      1,
      `${dependency} locked package record`,
    );
    requireLinePrefixCount(
      packageSections.snapshots,
      `'${dependency}@${lockedBase}`,
      1,
      `${dependency} locked snapshot record`,
    );
  }

  const expectedManifestEntry = `rusty-procgen-preflight = { git = "${sources.rustyProcgen.repository}", rev = "${sources.rustyProcgen.commit}", package = "rusty-procgen-preflight" }`;
  requireCount(
    cargoManifest,
    expectedManifestEntry,
    1,
    'canonical Procgen manifest entry',
  );

  const engineSource = `git+${sources.rustyEngine.repository}?rev=${sources.rustyEngine.commit}#${sources.rustyEngine.commit}`;
  for (const crate of ENGINE_RUST_CRATES) {
    const manifestEntry = `${crate} = { git = "${sources.rustyEngine.repository}", rev = "${sources.rustyEngine.commit}" }`;
    requireCount(
      cargoManifest,
      manifestEntry,
      1,
      `canonical ${crate} manifest entry`,
    );
    const records = cargoLock
      .split('[[package]]')
      .filter((block) =>
        new RegExp(`\\nname = "${crate}"\\n`).test(`\n${block}`),
      );
    if (records.length !== 1) {
      throw new Error(
        `${crate} locked package record expected 1, observed ${records.length}`,
      );
    }
    requireCount(
      records[0],
      `source = "${engineSource}"`,
      1,
      `canonical ${crate} locked source`,
    );
  }
  const procgenSource = `git+${sources.rustyProcgen.repository}?rev=${sources.rustyProcgen.commit}#${sources.rustyProcgen.commit}`;
  const procgenPackages = cargoLock
    .split('[[package]]')
    .filter((block) =>
      /\nname = "rusty-procgen-preflight"\n/.test(`\n${block}`),
    );
  if (procgenPackages.length !== 1) {
    throw new Error(
      `rusty-procgen-preflight locked package record expected 1, observed ${procgenPackages.length}`,
    );
  }
  requireCount(
    procgenPackages[0],
    `source = "${procgenSource}"`,
    1,
    'canonical Procgen locked source',
  );

  for (const [name, content] of [
    ['pnpm-lock.yaml', pnpmLock],
    ['rust/Cargo.lock', cargoLock],
  ]) {
    if (content.includes('/home/dev/') || content.includes('rusty-d20')) {
      throw new Error(`${name} contains a sibling or Rusty D20 dependency`);
    }
  }
}

function splitPnpmLock(lock) {
  const [beforeSnapshots, snapshots] = lock.split('\nsnapshots:\n');
  const [importer, packages] = beforeSnapshots?.split('\npackages:\n') ?? [];
  if (
    importer === undefined ||
    packages === undefined ||
    snapshots === undefined
  ) {
    throw new Error(
      'pnpm lock is missing importer, package, or snapshot sections',
    );
  }
  return { importer, packages, snapshots };
}

function requireCount(content, needle, expected, label) {
  const observed = content.split(needle).length - 1;
  if (observed !== expected) {
    throw new Error(`${label} expected ${expected}, observed ${observed}`);
  }
}

function requireLinePrefixCount(content, prefix, expected, label) {
  const observed = content
    .split('\n')
    .filter((line) => line.trimStart().startsWith(prefix)).length;
  if (observed !== expected) {
    throw new Error(`${label} expected ${expected}, observed ${observed}`);
  }
}

async function loadInputs() {
  return {
    sources: JSON.parse(await readFile('dependency-sources.json', 'utf8')),
    packageJson: JSON.parse(await readFile('package.json', 'utf8')),
    cargoManifest: await readFile('rust/Cargo.toml', 'utf8'),
    cargoLock: await readFile('rust/Cargo.lock', 'utf8'),
    pnpmLock: await readFile('pnpm-lock.yaml', 'utf8'),
  };
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  const inputs = await loadInputs();
  validateDependencySources(inputs);
  console.log(
    `Dependency sources passed: Engine ${inputs.sources.rustyEngine.commit}, Procgen ${inputs.sources.rustyProcgen.commit}`,
  );
}
