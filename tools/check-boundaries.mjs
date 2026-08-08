import { readFile, readdir } from 'node:fs/promises';
import { extname, join } from 'node:path';

const roots = ['apps', 'libs', 'rust', 'scripts', 'tools'];
const forbidden = [
  ['@rusty', '-d20/'].join(''),
  ['/home/dev/rusty', '-d20'].join(''),
  ['path = "../rusty', '-engine'].join(''),
  ['path = "../rusty', '-procgen'].join(''),
  ['file:../rusty', '-engine'].join(''),
  ['file:../rusty', '-procgen'].join(''),
];
const extensions = new Set([
  '.ts',
  '.mts',
  '.mjs',
  '.js',
  '.rs',
  '.toml',
  '.json',
  '.sh',
]);

for (const root of roots) {
  for (const file of await filesUnder(root)) {
    if (!extensions.has(extname(file))) continue;
    const content = await readFile(file, 'utf8');
    for (const needle of forbidden) {
      if (content.includes(needle)) {
        throw new Error(
          `${file} crosses a forbidden repository boundary with ${needle}`,
        );
      }
    }
    if (/from ['"]@rusty-roguelike\/.+\/src\//.test(content)) {
      throw new Error(`${file} deep-imports another browser package`);
    }
  }
}

for (const root of ['apps', 'libs']) {
  for (const file of await filesUnder(root)) {
    if (!new Set(['.ts', '.mts', '.js', '.mjs']).has(extname(file))) continue;
    const content = await readFile(file, 'utf8');
    if (content.includes('@rusty-engine/')) {
      throw new Error(
        `${file} imports a forbidden downstream Engine TypeScript package`,
      );
    }
  }
}

console.log('Rusty Roguelike repository boundaries passed');

async function filesUnder(root) {
  const result = [];
  async function visit(path) {
    for (const entry of await readdir(path, { withFileTypes: true })) {
      const child = join(path, entry.name);
      if (entry.isDirectory()) await visit(child);
      else result.push(child);
    }
  }
  await visit(root);
  return result;
}
