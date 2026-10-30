import { describe, expect, test } from 'bun:test';
import { apply } from '../src';
import type { Cmd } from '../src';
import { AG1, CMP_1, CMP_2, RUN_A, RUN_B, RUN_C, TASKS, U_AGENT_OWNER, U_OWNER, U_STRANGER, errCode, id, play, prng, wak } from './helpers';

// Differential property: the refusal path and the handler path must agree on WHO is calling. For seeded random commands with
// one field malformed or a prototype key and stray undeclared fields, the caller-level outcome and the renewal are what the
// caller rules say (ADR 0002, ADR 0003), computed here from the command type alone, not from the code under test.
const T = TASKS;
const base = play([[{ type: 'claim', runId: RUN_A, taskId: T[0] }, 0], [{ type: 'claim', runId: RUN_A, taskId: T[1] }, 0],
  [{ type: 'run_end', runId: RUN_C, computerId: CMP_2, outcome: 'completed' }, 1], [{ type: 'claim', runId: RUN_B, taskId: T[2] }, 2]]).state;
const NOW = 30_000;
const BAD: unknown[] = ['__proto__', 'constructor', 'toString', 'valueOf', 7, null, [], {}, 'x', undefined];
const GOOD: Record<string, unknown> = { taskId: T[0], title: 't', kind: 'chore', assignee: AG1, wakeId: wak(3), status: 'done', messageId: id('msg', 1), text: 'hi', summary: 's', runId: RUN_A, computerId: CMP_1, reason: 'r', outcome: 'completed', agentId: AG1, ownerId: U_OWNER.id, userId: U_STRANGER.id, staleMs: 0 };
const FIELDS: Record<string, string[]> = {
  claim: ['runId', 'taskId'], op: ['runId', 'taskId'], task_release: ['taskId'], run_stop: ['runId'], task_create: ['taskId', 'title', 'kind', 'wakeId'],
  task_update: ['taskId', 'status', 'wakeId'], message_post: ['messageId', 'text', 'summary', 'wakeId'], heartbeat: ['computerId'], wake_ack: ['wakeId', 'computerId'], run_end: ['runId', 'computerId', 'outcome'],
  // the seven types with no caller: a stray caller or runId must change nothing about them
  pod_create: ['ownerId'], member_add: ['userId'], change: ['agentId', 'wakeId'], run_claim: ['wakeId', 'computerId', 'runId'], agent_register: ['agentId', 'ownerId', 'computerId'], tick: [], boot: ['staleMs'],
};
const CALLERS: unknown[] = [{ kind: 'run', runId: RUN_A }, { kind: 'run', runId: RUN_C }, { kind: 'run', runId: id('run', 99) }, { kind: 'run', runId: 'constructor' },
  { kind: 'user', userId: U_OWNER.id }, { kind: 'user', userId: U_STRANGER.id }, { kind: 'user', userId: '__proto__' }, {}];
/** A `type` that is not the command's own string: arrays that coerce to it, numbers, null, objects, prototype keys. */
const BAD_TYPES = (t: string): unknown[] => [[t], 7, null, {}, [], ['change'], ['run_end'], 'constructor', '__proto__', 'toString'];
const WITH_CALLER = ['task_release', 'run_stop', 'task_create', 'task_update', 'message_post'];

/** The caller, by command type alone: a run for claim/op, the caller field for the five, nobody otherwise. */
function callerSpec(cmd: Record<string, unknown>): { kind: 'run' | 'user'; id: unknown } | null {
  const k = cmd.caller as { kind?: string; runId?: unknown; userId?: unknown } | undefined;
  if (cmd.type === 'claim' || cmd.type === 'op') return typeof cmd.runId === 'string' ? { kind: 'run', id: cmd.runId } : null;
  if (!WITH_CALLER.includes(cmd.type as string) || typeof k !== 'object' || k === null) return null;
  if (k.kind === 'run' && typeof k.runId === 'string') return { kind: 'run', id: k.runId };
  return k.kind === 'user' && typeof k.userId === 'string' ? { kind: 'user', id: k.userId } : null;
}

test('refusal path and handler path agree on the caller: outcome and renewal, over stray fields and bad input', () => {
  const r = prng(4242);
  const pick = <T2>(xs: T2[]): T2 => xs[Math.floor(r() * xs.length)];
  let judged = 0;
  for (let i = 0; i < 6000; i += 1) {
    const type = pick(Object.keys(FIELDS));
    const cmd: Record<string, unknown> = { type };
    for (const f of FIELDS[type]) cmd[f] = GOOD[f];
    if (type === 'change') Object.assign(cmd, { kind: 'ruling', refs: {} });
    if (WITH_CALLER.includes(type)) cmd.caller = pick(CALLERS);
    if (FIELDS[type].length > 0 && r() < 0.8) cmd[pick(FIELDS[type])] = pick(BAD);
    if (r() < 0.5) cmd[pick(['caller', 'runId', 'computerId'])] = pick([...CALLERS, RUN_B, CMP_2]);
    if (r() < 0.1) cmd.type = pick(BAD_TYPES(type));
    const spec = callerSpec(cmd);
    const res = apply(base, cmd as unknown as Cmd, NOW);
    const rep = res.out.find((o) => o.kind === 'reply');
    const code = rep?.kind === 'reply' && !rep.ok ? rep.error.code : null;
    if (typeof cmd.type !== 'string' || !Object.hasOwn(FIELDS, cmd.type)) expect([JSON.stringify(cmd), code]).toEqual([JSON.stringify(cmd), 'invalid_input']);
    const live = spec?.kind === 'run' && (spec.id === RUN_A || spec.id === RUN_B);
    const members = [U_OWNER.id, id('usr', 2)];
    const want = spec?.kind === 'run' && !live ? 'unauthorized' : live && type === 'run_stop' ? 'forbidden' : spec?.kind === 'user' && cmd.caller && !members.includes(spec.id as string) && (spec.id === U_STRANGER.id || BAD.includes(spec.id)) ? 'forbidden' : undefined;
    const tag = JSON.stringify(cmd);
    if (want !== undefined) expect([tag, code]).toEqual([tag, want]);
    else if (spec !== null) expect([tag, code !== 'unauthorized']).toEqual([tag, true]);
    // renewal: exactly the live caller's own claims move to now + 60 s; nobody else's lease is touched by a refused call
    const lease = (t: string) => res.state.claims[t]?.leaseUntil;
    const refused = code !== null;
    for (const [run, held, before] of [[RUN_A, [T[0], T[1]], 60_000], [RUN_B, [T[2]], 60_002]] as const) {
      const mine = live && spec?.id === run;
      if (refused) for (const t of held) expect([tag, t, lease(t)]).toEqual([tag, t, mine ? NOW + 60_000 : before]);
    }
    judged += 1;
  }
  expect(judged).toBe(6000);
});

describe('the reviewer\'s caller cases, as rows', () => {
  const ended = play([[{ type: 'run_end', runId: RUN_A, computerId: CMP_1, outcome: 'completed' }, 1]]).state;
  const user = (userId: string) => ({ kind: 'user', userId });
  test('an ended run\'s claim or op is unauthorized whatever else it carries: a stray caller, a bad task id (cases c, d, e, f)', () => {
    const rows = [
      { type: 'claim', runId: RUN_A, taskId: '__proto__', caller: user(U_STRANGER.id) }, { type: 'claim', runId: RUN_A, taskId: '__proto__', caller: user(U_OWNER.id) },
      { type: 'claim', runId: RUN_A, taskId: 7, caller: {} }, { type: 'op', runId: RUN_A, taskId: 'constructor', caller: {} },
    ] as unknown as Cmd[];
    for (const cmd of rows) expect([JSON.stringify(cmd), errCode(play([[cmd, 2]], ended).out)]).toEqual([JSON.stringify(cmd), 'unauthorized']);
  });
  const held = (cmd: unknown) => {
    const r = play([[{ type: 'claim', runId: RUN_A, taskId: T[0] }, 0], [{ type: 'claim', runId: RUN_B, taskId: T[1] }, 0], [cmd as Cmd, 50_000], [{ type: 'tick' }, 60_000]]);
    return Object.keys(r.state.claims).sort();
  };
  test('a refused claim renews its own run only: a stray user caller changes nothing (h), a stray run caller renews nobody else (j)', () => {
    expect(held({ type: 'claim', runId: RUN_A, taskId: '__proto__' })).toEqual([T[0]]);
    expect(held({ type: 'claim', runId: RUN_A, taskId: '__proto__', caller: user(U_OWNER.id) })).toEqual([T[0]]);
    expect(held({ type: 'claim', runId: RUN_A, taskId: 7, caller: {} })).toEqual([T[0]]);
    expect(held({ type: 'claim', runId: RUN_A, taskId: '__proto__', caller: { kind: 'run', runId: RUN_B } })).toEqual([T[0]]);
  });
  test('a heartbeat or wake_ack with a stray caller renews no run', () => {
    expect(held({ type: 'heartbeat', computerId: 7, caller: { kind: 'run', runId: RUN_A } })).toEqual([]);
    expect(held({ type: 'wake_ack', wakeId: 7, computerId: CMP_1, caller: { kind: 'run', runId: RUN_A } })).toEqual([]);
  });
  test('a live run\'s refused run_stop still renews its claims (MINE-A)', () => {
    expect(held({ type: 'run_stop', caller: { kind: 'run', runId: RUN_A }, runId: '__proto__' })).toEqual([T[0]]);
  });
  test('a member who is not an owner is judged as a member, so a malformed command is not forbidden (MINE-B)', () => {
    const cmd = { type: 'task_release', caller: user(U_AGENT_OWNER.id), taskId: '__proto__' } as unknown as Cmd;
    expect(errCode(play([[cmd, 1]]).out)).toBe('not_found');
  });
});

test('a non-string or prototype-key type is an unknown command: no throw, no state change, no renewal, for all 17 types', () => {
  const user = { kind: 'user', userId: U_OWNER.id };
  const ok: Record<string, unknown>[] = Object.keys(FIELDS).map((type) => {
    const cmd: Record<string, unknown> = { type };
    for (const f of FIELDS[type]) cmd[f] = GOOD[f];
    if (type === 'change') Object.assign(cmd, { kind: 'ruling', refs: {} });
    if (WITH_CALLER.includes(type)) cmd.caller = user;
    return cmd;
  });
  expect(ok.length).toBe(17);
  for (const cmd of ok) {
    for (const type of BAD_TYPES(cmd.type as string)) {
      const bad = { ...cmd, type, caller: { kind: 'run', runId: RUN_A } };
      const r = apply(base, bad as unknown as Cmd, NOW);
      expect([JSON.stringify(bad), errCode(r.out), r.out.length, r.state]).toEqual([JSON.stringify(bad), 'invalid_input', 1, base]);
    }
  }
});

describe('a stray caller or runId on a command that has none changes nothing, one row per type', () => {
  const NONE = ['pod_create', 'member_add', 'change', 'run_claim', 'agent_register', 'tick', 'boot'];
  const strays = [{ kind: 'run', runId: RUN_A }, { kind: 'run', runId: RUN_C }, { kind: 'run', runId: id('run', 99) }, { kind: 'user', userId: U_STRANGER.id }];
  for (const type of NONE) {
    test(type, () => {
      const cmd: Record<string, unknown> = { type };
      for (const f of FIELDS[type]) cmd[f] = GOOD[f];
      if (type === 'change') Object.assign(cmd, { kind: 'ruling', refs: {} });
      if (FIELDS[type].length > 0) cmd[FIELDS[type][0]] = 7; // malformed, so the command takes the refusal path, where callerOf is read
      const plain = apply(base, cmd as unknown as Cmd, NOW);
      for (const caller of strays) {
        const stray = { ...cmd, caller, ...(type === 'run_claim' ? {} : { runId: RUN_A }) };
        expect([type, JSON.stringify(caller), apply(base, stray as unknown as Cmd, NOW)]).toEqual([type, JSON.stringify(caller), plain]);
      }
    });
  }
});
