import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

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
  pnpmWorkspace,
  agentGuidance,
  design,
  sourceProvenance,
}) {
  validateDependencyDocumentation({
    'AGENTS.md': agentGuidance,
    'docs/design.md': design,
    'docs/source-provenance.md': sourceProvenance,
  });

  for (const [name, source] of Object.entries({
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
  for (const [name, content] of [
    ['package.json', JSON.stringify(packageJson)],
    ['pnpm-lock.yaml', pnpmLock],
    ['pnpm-workspace.yaml', pnpmWorkspace],
  ]) {
    if (content.includes('@rusty-engine/')) {
      throw new Error(`${name} contains a forbidden Engine TypeScript package`);
    }
  }

  const expectedManifestEntry = `rusty-procgen-preflight = { git = "${sources.rustyProcgen.repository}", rev = "${sources.rustyProcgen.commit}", package = "rusty-procgen-preflight" }`;
  requireCount(
    cargoManifest,
    expectedManifestEntry,
    1,
    'canonical Procgen manifest entry',
  );

  const facadeManifestEntry =
    'rusty-engine = { path = "../../rusty-engine/rust/crates/rusty-engine" }';
  requireCount(
    cargoManifest,
    facadeManifestEntry,
    1,
    'adjacent rusty-engine facade manifest entry',
  );
  for (const crate of ENGINE_RUST_CRATES) {
    if (new RegExp(`^${crate}\\s*=`, 'mu').test(cargoManifest)) {
      throw new Error(
        `${crate} must be reached through the rusty-engine facade`,
      );
    }
  }
  const facadeRecords = cargoLock
    .split('[[package]]')
    .filter((block) => /\nname = "rusty-engine"\n/.test(`\n${block}`));
  if (facadeRecords.length !== 1) {
    throw new Error(
      `rusty-engine locked package record expected 1, observed ${facadeRecords.length}`,
    );
  }
  if (/\nsource = /.test(facadeRecords[0])) {
    throw new Error('rusty-engine facade must resolve from the adjacent path');
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

function validateDependencyDocumentation(documents) {
  const staleEngineClaims = [
    /\bengine-source\.json\b/iu,
    /\bscripts\/engine-revision\b/iu,
    /\bCargo\.lock records the exact (?:Engine|resolved) revision\b/iu,
    /\bEngine's complete Rust facade follows rolling\b/iu,
    /\bno sibling path fallback/iu,
  ];

  for (const [name, content] of Object.entries(documents)) {
    const statements = content
      .split(/\n\s*\n/u)
      .flatMap((paragraph) =>
        paragraph.replace(/\n+/gu, ' ').split(/(?<=[.!?])(?:\s+|$)/u),
      )
      .map((statement) => statement.trim())
      .filter(Boolean);
    for (const statement of statements) {
      const mentionsEngine = /\bEngine\b/iu.test(statement);
      const pinsEngine =
        mentionsEngine && /\bpin(?:ned|ning|s)?\b/iu.test(statement);
      const namesEngineRevision =
        /\bEngine revision\b|\brevision (?:of|for) (?:the )?Engine\b|\bEngine(?:'s)? (?:source|identity)\b[^.!?]*\brevision\b|\brevision\b[^.!?]*\bEngine(?:'s)? (?:source|identity)\b/iu.test(
          statement,
        );
      const carriesEngineRevision =
        namesEngineRevision &&
        /\b(?:current|exact|identity|reviewed|source)\b|[0-9a-f]{7,40}/iu.test(
          statement,
        );
      const stale =
        pinsEngine ||
        carriesEngineRevision ||
        staleEngineClaims.some((pattern) => pattern.test(statement));
      const explicitHistory =
        name === 'docs/source-provenance.md' &&
        /\b(?:historical|history)\b/iu.test(statement);
      const explicitNegation = /\bEngine revision identity is not\b/iu.test(
        statement,
      );
      if (stale && !explicitHistory && !explicitNegation) {
        throw new Error(
          `${name} contains stale Engine pin or revision-carrier guidance: ${statement}`,
        );
      }
    }
  }
}

function requireCount(content, needle, expected, label) {
  const observed = content.split(needle).length - 1;
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
    pnpmWorkspace: await readFile('pnpm-workspace.yaml', 'utf8'),
    agentGuidance: await readFile('AGENTS.md', 'utf8'),
    design: await readFile('docs/design.md', 'utf8'),
    sourceProvenance: await readFile('docs/source-provenance.md', 'utf8'),
  };
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  const inputs = await loadInputs();
  validateDependencySources(inputs);
  console.log(
    `Dependency sources passed: adjacent Engine facade, Procgen ${inputs.sources.rustyProcgen.commit}`,
  );
}
