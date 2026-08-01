import { spawn } from 'node:child_process';

const address = process.env['RUSTY_ROGUELIKE_ADDRESS'] ?? '0.0.0.0:4417';
await run('pnpm', ['run', 'build']);
const host = spawn(
  'cargo',
  [
    'run',
    '--manifest-path',
    'rust/Cargo.toml',
    '-p',
    'rusty-roguelike',
    '--bin',
    'rusty-roguelike-host',
    '--',
    '--address',
    address,
  ],
  { stdio: 'inherit' },
);

for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, () => host.kill(signal));
}
host.on('exit', (code) => process.exit(code ?? 1));

function run(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { stdio: 'inherit' });
    child.on('error', reject);
    child.on('exit', (code) =>
      code === 0 ? resolve() : reject(new Error(`${command} exited ${code}`)),
    );
  });
}
