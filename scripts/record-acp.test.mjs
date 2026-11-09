// `record-acp.mjs --scrub <raw>` scrubs a raw log a refused scrub left behind. It must remove the raw file and nothing else:
// the directory holding that raw log belongs to whoever passed it in (it once deleted the repo root).
import { expect, test } from 'bun:test';
import { existsSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

test('--scrub keeps the directory holding the raw log', () => {
  const dir = mkdtempSync(join(tmpdir(), 'jmrec-scrub-test-'));
  const keepMe = join(dir, 'keep-me.txt');
  const raw = join(dir, 'raw.jsonl');
  const out = join(dir, 'out.jsonl');
  writeFileSync(keepMe, 'not ours to delete\n');
  writeFileSync(raw, JSON.stringify({ t: 'acp', dir: 'out', msg: { method: 'session/update', params: { update: { sessionUpdate: 'agent_message_chunk', content: { type: 'text', text: 'hello there' } } } } }) + '\n');
  const r = Bun.spawnSync([process.execPath, join(import.meta.dir, 'record-acp.mjs'), '--scrub', raw, out]);
  expect(r.exitCode).toBe(0);
  expect(existsSync(out)).toBe(true); // the scrubbed recording is written
  expect(existsSync(raw)).toBe(false); // the raw log goes
  expect(existsSync(dir)).toBe(true); // its directory stays
  expect(existsSync(keepMe)).toBe(true); // and so does everything else in it
});
