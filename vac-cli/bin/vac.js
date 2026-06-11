#!/usr/bin/env node
const { spawnSync } = require('node:child_process');
const path = require('node:path');
const fs = require('node:fs');

const candidates = [];
if (process.env.VAC_NATIVE_BINARY) candidates.push(process.env.VAC_NATIVE_BINARY);
candidates.push('vac');
candidates.push(path.resolve(__dirname, '..', '..', 'vac-rs', 'target', 'release', process.platform === 'win32' ? 'vac.exe' : 'vac'));

let last;
for (const bin of candidates) {
  if (bin.includes(path.sep) && !fs.existsSync(bin)) continue;
  const result = spawnSync(bin, process.argv.slice(2), { stdio: 'inherit' });
  last = result;
  if (!result.error) process.exit(result.status ?? 0);
  if (result.error.code !== 'ENOENT') {
    console.error(result.error.message);
    process.exit(1);
  }
}
console.error('VAC native binary not found. Build vac-rs first or set VAC_NATIVE_BINARY.');
if (last && last.error) console.error(last.error.message);
process.exit(127);
