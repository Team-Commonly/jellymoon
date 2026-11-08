// Records one real Claude Code run through the claude-agent-acp adapter (docs/development/record-acp.md).
//   bun scripts/record-acp.mjs [out.jsonl]                    (default daemon/testdata/acp/claude-slice.jsonl)
//   bun scripts/record-acp.mjs --scrub <raw.jsonl> [out.jsonl]  scrubs a raw log a refused scrub left behind; no new run
// It starts a LOCAL server (worker/dev.sh), a pod, the daemon and one claude-code agent, assigns the agent one task, and waits
// for the run to end. The daemon starts the adapter through e2e/testdata/acprec, which logs the ACP lines and the agent's MCP calls.
// The raw log is scrubbed by scripts/acp-scrub.mjs and only the scrubbed file is kept. Needs `make acp-tools` first, and this
// machine's normal Claude Code sign-in (the adapter reads it; nothing here does).
import { spawn, execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, existsSync, writeFileSync, unlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { scrub } from './acp-scrub.mjs';

const ADAPTER = { adapter: '@agentclientprotocol/claude-agent-acp', version: '0.99.1' };
const root = resolve(import.meta.dirname, '..');
const args = process.argv.slice(2);
const again = args[0] === '--scrub' ? resolve(args[1]) : null;
const out = resolve(args[again ? 2 : 0] ?? join(root, 'daemon/testdata/acp/claude-slice.jsonl'));
const tmp = again ? dirname(again) : mkdtempSync(join(tmpdir(), 'jmrec-'));
const procs = [];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function run(cmd, args, opts = {}) {
  const p = spawn(cmd, args, { detached: true, stdio: ['ignore', 'pipe', 'pipe'], ...opts });
  p.text = '';
  for (const s of [p.stdout, p.stderr]) s.on('data', (d) => { p.text += d; });
  procs.push(p);
  return p;
}

async function until(what, ms, f) {
  for (const end = Date.now() + ms; Date.now() < end; await sleep(100)) { const v = await f(); if (v) return v; }
  throw new Error(`timed out waiting for ${what}`);
}

async function api(base, path, token, body) {
  const res = await fetch(base + path, { method: 'POST', headers: { 'Content-Type': 'application/json', ...(token ? { Authorization: `Bearer ${token}` } : {}) }, body: JSON.stringify(body) });
  if (!res.ok) throw new Error(`POST ${path}: HTTP ${res.status}`);
  return res.json();
}

/** The local server, a user and a pod. */
async function startServer() {
  const port = 20000 + Math.floor(Math.random() * 20000), base = `http://127.0.0.1:${port}`;
  run('bash', [join(root, 'worker/dev.sh')], { env: { ...process.env, PORT: String(port), ENV: 'local', JM_DATA_DIR: join(tmp, 'data') } });
  await until('/healthz', 90000, async () => (await fetch(`${base}/healthz`).catch(() => null))?.ok);
  const { token } = await api(base, '/__test/session', '', { name: 'olivia' });
  const pod = await api(base, '/v1/pods', token, { name: 'slice' });
  return { base, token, podId: pod.id };
}

/** The daemon, with the adapter behind the tee as its claude-code program, and one agent. The daemon's directory is empty. */
async function startDaemon(s, raw) {
  const bin = join(tmp, 'jellymoon'), tee = join(tmp, 'acprec'), work = join(tmp, 'work');
  execFileSync('go', ['build', '-o', bin, './daemon/cmd/jellymoon'], { cwd: root });
  execFileSync('go', ['build', '-o', tee, './e2e/testdata/acprec'], { cwd: root });
  mkdirSync(work);
  const adapter = join(root, '.tools/acp/node_modules/.bin/claude-agent-acp');
  const env = { PATH: process.env.PATH, HOME: process.env.HOME, USER: process.env.USER, LANG: process.env.LANG ?? 'en_US.UTF-8', TMPDIR: tmp, JELLYMOON_HOME: join(tmp, 'jm'), JELLYMOON_SERVER: s.base,
    JELLYMOON_HARNESS_CLAUDE_CODE: `${tee} ${raw} ${adapter}` };
  const daemon = run(bin, ['daemon', 'start', '--foreground'], { cwd: work, env: { ...env, JELLYMOON_USER_TOKEN: s.token } });
  await until('the stored computer', 15000, () => existsSync(join(env.JELLYMOON_HOME, 'computer.json')));
  const added = execFileSync(bin, ['agent', 'add', 'claude-code', '--pod', s.podId, '--name', 'claude-one'], { env, encoding: 'utf8' });
  return { daemon, work, agentId: /agt_[0-9A-Z]{26}/.exec(added)[0] };
}

/** A person watches the pod; once the agent is online they assign it one task, which wakes it. */
async function assign(s, agentId) {
  const ws = new WebSocket(`${s.base.replace('http', 'ws')}/v1/pods/${s.podId}/ws`, { headers: { Authorization: `Bearer ${s.token}` } });
  const seen = [];
  ws.onmessage = (e) => seen.push(JSON.parse(e.data));
  await until('the socket', 15000, () => seen.some((f) => f.type === 'hello'));
  const send = (type, data) => ws.send(JSON.stringify({ v: 1, type, podId: s.podId, data }));
  send('subscribe', { sinceSeq: 0 });
  await until('the agent online', 20000, () => seen.some((f) => /"online"/.test(JSON.stringify(f))));
  send('task.create', { title: 'Say hello to the pod', kind: 'chore', assignee: agentId });
  return ws;
}

/** The scrubbed recording is the only file kept; the raw log goes once it is written. */
function keep(raw) {
  const swaps = [[join(tmp, 'work'), '/work'], [tmp, '/tmp/rec'], [root, '/repo'], [process.env.HOME, '/sandbox/home']];
  let text;
  try { text = scrub(readFileSync(raw, 'utf8'), { ...ADAPTER, date: new Date().toISOString().slice(0, 10) }, swaps); } catch (e) { throw new Error(`${e.message}\nthe raw log is kept at ${raw}`); }
  writeFileSync(out, text);
  unlinkSync(raw);
}

async function main() {
  if (again) return keep(again);
  if (!existsSync(join(root, '.tools/acp/node_modules/.bin/claude-agent-acp'))) throw new Error('the adapter is not installed: run `make acp-tools`');
  const raw = join(tmp, 'raw.jsonl');
  const s = await startServer();
  const d = await startDaemon(s, raw);
  const ws = await assign(s, d.agentId);
  await until('the run to end', 240000, () => /run \S+ ended/.test(d.daemon.text));
  await sleep(1500);
  ws.close();
  console.log(d.daemon.text.split('\n').filter((l) => /run \S+ (starting|ended)/.test(l)).join('\n'));
  keep(raw);
}

try { await main(); } finally {
  for (const p of procs) { try { process.kill(-p.pid, 'SIGKILL'); } catch { /* already gone */ } }
}
