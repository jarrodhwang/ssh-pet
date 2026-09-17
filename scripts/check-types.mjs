import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
const directory = mkdtempSync(join(tmpdir(), 'droplet-types-'));
try {
  execFileSync('cargo', ['run', '-q', '-p', 'droplet-cli', '--', 'export-types', directory], { stdio: 'inherit' });
  const expected = readdirSync(directory).sort();
  const current = readdirSync('src/generated').sort();
  if (JSON.stringify(expected) !== JSON.stringify(current)) throw new Error('Generated type files differ. Run npm run types.');
  for (const file of expected) {
    if (readFileSync(join(directory, file), 'utf8') !== readFileSync(resolve('src/generated', file), 'utf8')) {
      throw new Error(`${file} differs from the Rust protocol. Run npm run types.`);
    }
  }
  console.log('TypeScript contracts match the Rust protocol.');
} finally { rmSync(directory, { recursive: true, force: true }); }
