import { describe, expect, test } from 'bun:test';
import { apply, nthWakeId } from '../src';
import type { Cmd } from '../src';
import { AG1, AG3, AG_UNREG, CMP_1, CMP_2, RUN_A, RUN_B, TASKS, U_AGENT_OWNER, U_OWNER, errCode, events, id, lastReply, play, runN, toComputer, wak, world } from './helpers';

const [T1, T2] = TASKS;
const call = { kind: 'user' as const, userId: U_OWNER.id };
/** Task 10+n, assigned to an agent: the core writes the task_assigned wake itself. */
const assign = (n: number, assignee = AG1, wakeId = wak(n)): Cmd => ({ type: 'task_create', caller: call, taskId: id('tsk', 10 + n), title: `T${n}`, kind: 'chore', assignee, wakeId });
const say = (n: number, text: string, caller: Cmd extends infer _ ? { kind: 'user'; userId: string } | { kind: 'run'; runId: string } : never = call): Cmd => ({ type: 'message_post', caller, messageId: id('msg', n), text, wakeId: wak(n) });
const ruling = (n: number): Cmd => ({ type: 'change', kind: 'ruling', agentId: AG1, refs: { decisionId: id('dec', 1) }, wakeId: wak(n) });
const hb = (computerId: string): Cmd => ({ type: 'heartbeat', computerId });
const claimWake = (n: number, computerId = CMP_1, r = 1): Cmd => ({ type: 'run_claim', wakeId: wak(n), computerId, runId: runN(r) });
const wakes = (out: ReturnType<typeof play>['out']) => toComputer(out).filter((o) => o.event.type === 'run.wake');
const stops = (out: ReturnType<typeof play>['out']) => toComputer(out).filter((o) => o.event.type === 'run.stop');
const wakeReasons = (out: ReturnType<typeof play>['out']) => wakes(out).map((o) => (o.event.type === 'run.wake' ? o.event.data.reason : ''));

describe('a change writes a wake only for the agent it addresses', () => {
  test('a task created for the agent writes one wake and sends run.wake to its computer only', () => {
    const r = play([[assign(1), 1000]]);
    expect(r.out.filter((o) => o.kind === 'to-computer')).toEqual([
      { kind: 'to-computer', computerId: CMP_1, event: { v: 1, type: 'run.wake', podId: r.state.podId, ts: '1970-01-01T00:00:01.000Z', data: { wakeId: wak(1), agentId: AG1, reason: 'task_assigned', refs: { taskId: id('tsk', 11) } } } },
    ]);
    expect(events(r.out).map((e) => e.type)).toEqual(['task.created']);
    expect(errCode(r.out)).toBeNull();
  });

  test('task_update to a new assignee wakes that agent; the same assignee, or an unregistered one, wakes nobody', () => {
    const upd = (assignee: string): Cmd => ({ type: 'task_update', caller: call, taskId: T1, assignee, wakeId: wak(1) });
    expect(wakeReasons(play([[upd(AG3), 0]]).out)).toEqual(['task_assigned']);
    expect(wakes(play([[upd(AG1), 0]]).out)).toEqual([]);
    expect(wakes(play([[upd(AG_UNREG), 0]]).out)).toEqual([]);
  });

  test('an @mention wakes the agent; a ruling is decision_ruled', () => {
    expect(wakeReasons(play([[say(1, 'ping @agent-1 please'), 0]]).out)).toEqual(['mention']);
    expect(wakeReasons(play([[ruling(1), 0]]).out)).toEqual(['decision_ruled']);
    const m = wakes(play([[say(1, 'ping @agent-1'), 0]]).out)[0].event;
    expect(m.type === 'run.wake' && m.data.refs).toEqual({ messageId: id('msg', 1) });
  });

  test('mentions match registered names exactly, and only outside code spans', () => {
    for (const text of ['hello there', '@agent-10 hi', '@agent hi', 'a@agent-1 hi', '`@agent-1` hi', '@Agent-1']) {
      expect(wakes(play([[say(1, text), 0]]).out)).toEqual([]);
    }
    expect(wakeReasons(play([[say(1, 'cc `x` @agent-1.'), 0]]).out)).toEqual(['mention']);
    expect(wakes(play([[say(1, '@agent-1 @agent-3 @agent-1'), 0]]).out).map((o) => o.computerId)).toEqual([CMP_1, CMP_2]);
  });

});

describe('mentions, run callers and validation', () => {
  test('mentions follow CommonMark: spans of any length, across a newline, fences; an escape or an unclosed backtick is literal', () => {
    const quiet = ['``@agent-1``', '`` @agent-1 ``', 'a `x\n@agent-1\ny` b', '```\n@agent-1', '~~~\n@agent-1\n~~~', '  ```js\n@agent-1\n```', '````\n```\n@agent-1\n````', '``a` @agent-1 `b``'];
    for (const text of quiet) expect([text, wakes(play([[say(1, text), 0]]).out)]).toEqual([text, []]);
    for (const text of ['\\`@agent-1\\`', '`@agent-1 with no closer', 'see ```\n@agent-1\n``` ok', '`a` @agent-1 `b`', '``` x`\n@agent-1']) {
      expect([text, wakeReasons(play([[say(1, text), 0]]).out)]).toEqual([text, ['mention']]);
    }
  });

  test('a run never wakes its own agent, through task_create, task_update or message_post; it does wake another', () => {
    const caller = { kind: 'run' as const, runId: RUN_A };
    const mk = (assignee: string): Cmd => ({ type: 'task_create', caller, taskId: id('tsk', 30), title: 't', kind: 'chore', assignee, wakeId: wak(1) });
    const up = (assignee: string): Cmd => ({ type: 'task_update', caller, taskId: T1, assignee, wakeId: wak(1) });
    for (const f of [mk, up]) {
      expect(wakes(play([[f(AG1), 0]]).out)).toEqual([]);
      expect(wakeReasons(play([[f(AG3), 0]]).out)).toEqual(['task_assigned']);
    }
    expect(wakes(play([[say(1, '@agent-1', caller), 0]]).out)).toEqual([]);
  });

  test('task_update refuses status open', () => {
    expect(errCode(play([[{ type: 'task_update', caller: call, taskId: T1, status: 'open' as never, wakeId: wak(1) }, 0]]).out)).toBe('invalid_input');
  });

});

describe('input an adapter could get wrong', () => {
  test('a malformed or exhausted wakeId, a non-string text, a bad now, a bad staleMs and an unknown type are refusals that change nothing', () => {
    const top = `wak_${'0'.repeat(10)}${'Z'.repeat(16)}`;
    const bad = ['', 'wak_1', wak(1).replace('wak', 'run'), `${wak(1).slice(0, -1)}U`, `${wak(1).slice(0, -1)}a`];
    const base = world();
    const refused = (cmd: Cmd, now = 0) => {
      const r = apply(base, cmd, now);
      expect([JSON.stringify(cmd), errCode(r.out), r.out.length, r.state]).toEqual([JSON.stringify(cmd), 'invalid_input', 1, base]);
    };
    for (const wakeId of bad) {
      refused({ ...assign(1), wakeId } as Cmd);
      refused({ type: 'task_update', caller: call, taskId: T1, status: 'done', wakeId });
      refused({ ...say(1, 'hi'), wakeId } as Cmd);
      refused({ type: 'change', kind: 'ruling', agentId: AG1, refs: {}, wakeId });
    }
    refused({ ...say(1, '@agent-1 @agent-3'), wakeId: top } as Cmd);
    refused({ ...say(1, 'hi'), text: 5 as never } as Cmd);
    refused({ type: 'tick' }, 1.5);
    refused({ type: 'tick' }, Number.NaN);
    refused({ type: 'tick' }, -1);
    for (const staleMs of [-1, 60_001, 1.5, Number.NaN]) refused({ type: 'boot', staleMs });
    refused({ type: 'nope' } as never);
    refused({ type: 'constructor' } as never);
    expect(wakes(play([[{ ...say(1, '@agent-1'), wakeId: top } as Cmd, 0]]).out).length).toBe(1);
  });

  test('an ambient message writes no wake', () => {
    const r = play([[say(1, 'good morning'), 0]]);
    expect(r.state.wakes).toEqual({});
    expect(events(r.out).map((e) => e.type)).toEqual(['message.created']);
  });

  test('a run\'s message never wakes its own agent, but does wake another', () => {
    const own = play([[say(1, '@agent-1 note to self', { kind: 'run', runId: RUN_A }), 0]]);
    expect(wakes(own.out)).toEqual([]);
    expect(wakes(play([[say(1, '@agent-3 your turn', { kind: 'run', runId: RUN_A }), 0]]).out).length).toBe(1);
  });

  test('an unaddressed kind writes nothing; an unregistered agent gets no held wake', () => {
    const r = play([[{ type: 'change', kind: 'message' as never, agentId: AG1, refs: {}, wakeId: wak(1) }, 0], [assign(1, AG_UNREG), 1], [{ type: 'agent_register', agentId: AG_UNREG, ownerId: U_AGENT_OWNER.id, computerId: CMP_1 }, 2]]);
    expect(r.state.wakes).toEqual({});
    expect(r.out.filter((o) => o.kind === 'to-computer')).toEqual([]);
  });

  test('the message and task events carry the next seq, and the message is stored as fyi/done', () => {
    const r = play([[say(1, 'first line\nsecond'), 5], [assign(1), 6]]);
    expect(events(r.out).map((e) => e.seq)).toEqual([1, 2]);
    const m = events(r.out)[0];
    expect(m.type === 'message.created' && m.data).toMatchObject({ id: id('msg', 1), seq: 1, kind: 'fyi', status: 'done', summary: 'first line', body: 'first line\nsecond', author: U_OWNER });
  });
});

describe('coalescing', () => {
  test('three assignments in a row leave one pending wake, delivered once, under the first wakeId', () => {
    const r = play([[assign(1), 0], [assign(2), 1], [say(1, '@agent-1 look'), 2]]);
    expect(wakes(r.out).length).toBe(1);
    expect(Object.values(r.state.wakes).length).toBe(1);
    expect(r.state.wakes[AG1]).toMatchObject({ id: wak(1), refs: { taskId: id('tsk', 11), messageId: id('msg', 1) } });
  });

  test('a repeated ref is deduped, and the first ref wins where the schema holds only one', () => {
    const r = play([[assign(1), 0], [assign(2), 1], [assign(3), 2]]);
    expect(r.state.wakes[AG1].refs).toEqual({ taskId: id('tsk', 11) });
  });

  test('wakes for different agents do not coalesce', () => {
    const r = play([[assign(1), 0], [assign(2, AG3), 1]]);
    expect(Object.keys(r.state.wakes).sort()).toEqual([AG1, AG3]);
    expect(toComputer(r.out).map((o) => o.computerId)).toEqual([CMP_1, CMP_2]);
  });

  test('once a run_claim consumes the wake, the next change starts a new one', () => {
    const r = play([[assign(1), 0], [claimWake(1), 1], [assign(2), 2]]);
    expect(r.state.wakes[AG1].id).toBe(wak(2));
    expect(wakes(r.out).length).toBe(2);
  });
});

describe('resend', () => {
  const sent = play([[assign(1), 0]]).state;
  const at = (cmd: Cmd, now: number) => play([[cmd, now]], sent).out;

  test('boundary: 59 999 ms resends nothing, 60 000 ms resends to the next heartbeat', () => {
    expect(wakes(at(hb(CMP_1), 59_999))).toEqual([]);
    expect(wakes(at(hb(CMP_1), 60_000)).length).toBe(1);
  });

  test('a heartbeat from another computer resends nothing, and neither does a tick', () => {
    expect(wakes(at(hb(CMP_2), 90_000))).toEqual([]);
    expect(wakes(at({ type: 'tick' }, 90_000))).toEqual([]);
  });

  test('an acked wake with no run_claim is re-sent like an un-acked one: nothing at 59 999 ms, a resend at 60 000 ms', () => {
    const ack: Cmd = { type: 'wake_ack', wakeId: wak(1), computerId: CMP_1 };
    expect(wakes(play([[ack, 10], [hb(CMP_1), 59_999]], sent).out)).toEqual([]);
    expect(wakes(play([[ack, 10], [hb(CMP_1), 60_000]], sent).out).length).toBe(1);
  });

  test('a resend restarts the 60 s wait', () => {
    const r = play([[hb(CMP_1), 60_000], [hb(CMP_1), 100_000], [hb(CMP_1), 120_000]], sent);
    expect(wakes(r.out).length).toBe(2);
  });

  test('wake_ack from the wrong computer is ignored', () => {
    const r = play([[{ type: 'wake_ack', wakeId: wak(1), computerId: CMP_2 }, 10], [hb(CMP_1), 60_000]], sent);
    expect(wakes(r.out).length).toBe(1);
  });

  test('a claimed wake is gone, so nothing is resent', () => {
    expect(wakes(play([[claimWake(1), 5], [hb(CMP_1), 70_000]], sent).out)).toEqual([]);
  });
});

describe('run.stop resend', () => {
  const stopped = play([[{ type: 'run_stop', caller: { kind: 'user', userId: U_OWNER.id }, runId: RUN_A, reason: 'hung' }, 1000]]).state;

  test('re-sent on every heartbeat from the hosting computer, with no wait, until run_end', () => {
    const r = play([[hb(CMP_1), 1001], [hb(CMP_1), 1002], [{ type: 'run_end', runId: RUN_A, computerId: CMP_1, outcome: 'cancelled' }, 1003], [hb(CMP_1), 1004]], stopped);
    expect(stops(r.out).length).toBe(2);
    expect(stops(r.out)[0]).toMatchObject({ computerId: CMP_1, event: { type: 'run.stop', data: { runId: RUN_A, reason: 'hung' } } });
  });

  test('a heartbeat from another computer re-sends nothing', () => {
    expect(stops(play([[hb(CMP_2), 1001]], stopped).out)).toEqual([]);
  });
});

describe('run_claim', () => {
  test('creates the run for the agent on the hosting computer, consumes the wake and caps the token at 2 h', () => {
    const r = play([[assign(1), 0], [claimWake(1), 5000]]);
    expect(r.state.runs[runN(1)]).toMatchObject({ agent: { id: AG1 }, owner: U_AGENT_OWNER, computerId: CMP_1, startedAt: 5000, status: 'active' });
    expect(r.state.wakes).toEqual({});
    const reply = lastReply(r.out);
    expect(reply?.kind === 'reply' && reply.ok && reply.result).toMatchObject({ runId: runN(1), expiresAt: '1970-01-01T02:00:05.000Z', reason: 'task_assigned' });
    expect(events(r.out).length).toBe(1);
  });

  test('the new run is live: it can claim a task', () => {
    const r = play([[assign(1), 0], [claimWake(1), 1], [{ type: 'claim', runId: runN(1), taskId: T1 }, 2]]);
    expect(r.state.claims[T1].runId).toBe(runN(1));
  });

  test('a run id that exists, live or ended, is refused and the wake stays', () => {
    const ended = play([[{ type: 'run_end', runId: RUN_A, computerId: CMP_1, outcome: 'completed' }, 1]]).state;
    for (const from of [world(), ended]) {
      const r = play([[assign(1), 2], [{ type: 'run_claim', wakeId: wak(1), computerId: CMP_1, runId: RUN_A }, 3]], from);
      expect(errCode(r.out)).toBe('invalid_input');
      expect(r.state.wakes[AG1].id).toBe(wak(1));
    }
  });

  test('an unknown or already consumed wake gets not_found', () => {
    expect(errCode(play([[claimWake(7), 0]]).out)).toBe('not_found');
    expect(errCode(play([[assign(1), 0], [claimWake(1), 1], [claimWake(1, CMP_1, 2), 2]]).out)).toBe('not_found');
  });

  test('a computer that does not host the agent gets forbidden and the wake stays', () => {
    const r = play([[assign(1), 0], [claimWake(1, CMP_2), 1]]);
    expect(errCode(r.out)).toBe('forbidden');
    expect(r.state.wakes[AG1].id).toBe(wak(1));
    expect(Object.keys(r.state.runs).includes(runN(1))).toBe(false);
  });
});

describe('wake ids are minted by the adapter', () => {
  test('one message mentioning three agents opens three distinct valid wak_ ULIDs, the k-th being nthWakeId(seed, k)', () => {
    const r = play([[say(5, '@agent-1 @agent-2 @agent-3'), 0]]);
    const ids = wakes(r.out).map((o) => (o.event.type === 'run.wake' ? o.event.data.wakeId : ''));
    expect(ids).toEqual([wak(5), wak(6), wak(7)]);
    expect(ids.every((x) => /^wak_[0-9A-HJKMNP-TV-Z]{26}$/.test(x))).toBe(true);
  });
  test('a coalesced wake does not use up a k', () => {
    expect(wakes(play([[assign(1), 0], [say(5, '@agent-1 @agent-3'), 1]]).out).map((o) => o.event.type === 'run.wake' && o.event.data.wakeId)).toEqual([wak(1), wak(5)]);
  });
  test('nthWakeId is pure, carries across digits, and refuses a malformed seed or an overflow', () => {
    const seed = `wak_${'0'.repeat(24)}Z1`;
    expect([nthWakeId(seed, 0), nthWakeId(seed, 1), nthWakeId(seed, 31)]).toEqual([seed, `wak_${'0'.repeat(24)}Z2`, `wak_${'0'.repeat(23)}100`]);
    const top = `wak_${'0'.repeat(10)}${'Z'.repeat(16)}`;
    expect([nthWakeId(top, 0), nthWakeId(top, 1)]).toEqual([top, null]);
    expect([nthWakeId(`wak_${'1'.repeat(10)}${'Z'.repeat(16)}`, 1), nthWakeId('x', 0), nthWakeId(seed, -1), nthWakeId(seed, 0.5)]).toEqual([null, null, null, null]);
  });
  test('two pods sharing a computer emit the ids they were given, so no wakeId repeats; a coalesced command ignores its id', () => {
    const other = { ...world(), podId: id('pod', 2) };
    const a = play([[assign(1), 0], [assign(2), 1]]);
    const b = play([[assign(3, AG1, wak(7)), 0]], other);
    const ids = [...wakes(a.out), ...wakes(b.out)].map((o) => (o.event.type === 'run.wake' ? o.event.data.wakeId : ''));
    expect(ids).toEqual([wak(1), wak(7)]);
    expect(a.state.wakes[AG1].id).toBe(wak(1));
  });
});

describe('agent_register', () => {
  test('moving an agent re-addresses its pending wake to the new computer; the old computer hears nothing more', () => {
    const base = play([[assign(1), 0], [{ type: 'wake_ack', wakeId: wak(1), computerId: CMP_1 }, 1]]);
    const move = play([[{ type: 'agent_register', agentId: AG1, ownerId: U_AGENT_OWNER.id, computerId: CMP_2 }, 5]], base.state);
    expect(wakes(move.out).map((o) => [o.computerId, o.event.type === 'run.wake' && o.event.data.wakeId])).toEqual([[CMP_2, wak(1)]]);
    expect(wakes(play([[hb(CMP_1), 70_000]], move.state).out)).toEqual([]);
    expect(wakes(play([[hb(CMP_2), 65_000]], move.state).out).length).toBe(1);
    expect(wakes(play([[{ type: 'agent_register', agentId: AG1, ownerId: U_AGENT_OWNER.id, computerId: CMP_2 }, 6]], move.state).out)).toEqual([]);
  });

  test('moves hosting: a later wake goes to the new computer', () => {
    const r = play([[{ type: 'agent_register', agentId: AG1, ownerId: U_AGENT_OWNER.id, computerId: CMP_2 }, 0], [assign(1), 1]]);
    expect(wakes(r.out).map((o) => o.computerId)).toEqual([CMP_2]);
    expect(world().agents[AG1].computerId).toBe(CMP_1);
  });
});
