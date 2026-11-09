// Plants one line per class of thing that must never reach the committed recording. Each must come out scrubbed (the secret is gone
// from the output) or be refused (scrub throws). A line that comes out with its secret intact is the failure.
import { expect, test } from 'bun:test';
import { scrub } from './acp-scrub.mjs';

const H = { adapter: 'x', version: '0' };
const update = (text, messageId) => ({ sessionUpdate: 'agent_message_chunk', content: { type: 'text', text }, ...(messageId ? { messageId } : {}) });
const chunk = (text, messageId) => JSON.stringify({ t: 'acp', dir: 'out', msg: { method: 'session/update', params: { update: update(text, messageId) } } });
const note = (text) => JSON.stringify({ t: 'acp', dir: 'out', msg: { method: 'x', params: { text } } });

/** The planted secret must be gone from the output, or the scrub must refuse. */
function holds(raw, secret) {
  let out;
  try { out = scrub(raw, H, []); } catch { return; }
  expect(out).not.toContain(secret);
}

const tail = 'A1b2C3d4E5f6G7h8I9j0K1l2M3n4O5p6Q7r8S9t0U1v';
const planted = {
  'a run token': ['jmr_' + tail, 'A1b2C3d4E5f6'],
  'a user token': ['jmu_' + tail, 'A1b2C3d4E5f6'],
  'a computer token': ['jmc_' + tail, 'A1b2C3d4E5f6'],
  'a bearer': ['Authorization: Bearer abcdEFGH12345678.zz', 'abcdEFGH12345678'],
  'an Anthropic message id': ['msg_011CgqnDp1afj3X8sYoyuoDn', '011CgqnDp1afj3X8'],
  'an org id': ['org_9f8e7d6c5b4a3210', '9f8e7d6c5b4a3210'],
  'a github token': ['ghp' + '_16C7e42F292c6912E7710c838347Ae178B4a', '16C7e42F292c6912E'],
  'an email': ['someone@example.org', 'someone@example'],
  'a home path': ['/Us' + 'ers/jane/work/app', 'jane'],
  'a volumes path': ['/Volumes/Backup/secret/notes.txt', 'Backup'],
  'a schemeless local host': ['build-box.local:8080', 'build-box'],
  'a schemeless public host': ['internal.corp-name.io/path', 'corp-name'],
  'a url': ['https://example.net/a', 'example.net'],
};

for (const [what, [text, secret]] of Object.entries(planted)) {
  test(`${what} does not get through`, () => holds(note(`see ${text} here`), secret));
  test(`${what} in a message id does not get through`, () => holds(chunk('x', text), secret));
}

test('an id split across streamed chunks does not get through', () => {
  const raw = [chunk('I claimed run_01M6XZZ5FD8NN'), chunk('0HZV23EDXY554 and tsk_01M6XZZ5F56C'), chunk('JDK3H8B7T6PJA')].join('\n');
  for (const part of ['0HZV23EDXY554', 'JDK3H8B7T6PJA', '01M6XZZ5FD8NN', '01M6XZZ5F56C']) holds(raw, part);
});

test('streamed text is replaced and the line stays', () => {
  const out = scrub(chunk('hello there'), H, []).split('\n');
  expect(out.length).toBe(2);
  expect(out[1]).toContain('agent_message_chunk');
  expect(out[1]).not.toContain('hello there');
});

test("Codex's turn metadata keeps its keys but not this machine's settings", () => {
  const meta = { session_id: 'a', model: 'some-model', reasoning_effort: 'high', sandbox: 'seatbelt', auto_review_enabled: true, turn_started_at_unix_ms: 1794199983781, codex_version: '0.166.0' };
  const line = JSON.stringify({ t: 'acp', dir: 'out', msg: { method: 'x', params: { _meta: { 'x-codex-turn-metadata': meta } } } });
  const out = scrub(line, H, []);
  for (const gone of ['some-model', 'seatbelt', '0.166.0', '1794199983781', 'high']) expect(out).not.toContain(gone);
  const kept = JSON.parse(out.split('\n')[1]).msg.params._meta['x-codex-turn-metadata'];
  expect(Object.keys(kept)).toEqual(Object.keys(meta));
  expect(kept.model).toBe('[model]');
});

test('a placeholder id and an ordinary line pass', () => {
  const out = scrub(note('pod_00000000000000000000000002 at /work and 127.0.0.1:1 or application/json'), H, []);
  expect(out).toContain('pod_00000000000000000000000001');
});
