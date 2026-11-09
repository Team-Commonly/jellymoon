import { describe, expect, test } from 'bun:test';
import type { AskContext, Message, PostInput } from '../../gen/ts/protocol';
import { citedIds, numbersIn, ungrounded } from '../src';
import type { Cmd, Evidence } from '../src';
import { RUN_A, RUN_B, U_OWNER, errCode, events, id, play, world } from './helpers';

// Grounding (F25): what a number is, and when a claim's evidence grounds it.
const nums = (s: string) => numbersIn(s).map((n) => n.sign + n.digits);

describe('what counts as a number', () => {
  test('integers, decimals and versions are one number each; a trailing full stop is not part of one', () => {
    expect(nums('212 of 212 tests')).toEqual(['212', '212']);
    expect(nums('Release 4.2. Then 1.2.3 and 0.5')).toEqual(['4.2', '1.2.3', '0.5']);
    expect(nums('no digits here, twelve or a dozen')).toEqual([]);
    expect(nums('')).toEqual([]);
  });

  test('a comma is a thousands separator only in the 1,234 shape and is dropped; any other comma separates numbers', () => {
    expect(nums('1,024 files, 12,345,678.9 bytes')).toEqual(['1024', '12345678.9']);
    expect(nums('pick 1,2,3 or 1,23 or 1234,567')).toEqual(['1', '2', '3', '1', '23', '1234', '567']);
    expect(nums('3.14,2.71')).toEqual(['3.14', '2.71']);
  });

  test('a sign belongs to the number only when nothing word-like comes before it', () => {
    expect(nums('delta -3, gain +4, minus −5, (-6)')).toEqual(['-3', '+4', '-5', '-6']);
    expect(nums('2026-11-09 and items 3-5 and a-7 and x_+8 and 1.-9')).toEqual(['2026', '11', '09', '3', '5', '7', '8', '1', '9']);
    expect(nums('range 10 - 20')).toEqual(['10', '20']);
  });

  test('a percent or unit is not part of the number; digits inside a word still are', () => {
    expect(nums('87.5% in 10ms over 500MB')).toEqual(['87.5', '10', '500']);
    expect(nums('build b98213, sha256, v2')).toEqual(['98213', '256', '2']);
  });

  test('any Unicode decimal digit is a digit, kept as written', () => {
    expect(nums('２１２ and ٣')).toEqual(['２１２', '٣']);
  });
});

const TC = id('tc', 1), TC_B = id('tc', 2), MSG = id('msg', 1);
const ev = (digest: string, msg = { summary: 'Coverage report', body: 'Coverage is 87.5% across 1,024 files; delta -3.' }): Evidence => ({
  toolCalls: { [TC]: { runId: RUN_A, digest }, [TC_B]: { runId: RUN_B, digest } }, messages: { [MSG]: msg },
});
const claim = (text: string, toolCalls?: string[], messages?: string[]) => ({ text, ...(toolCalls ? { toolCalls } : {}), ...(messages ? { messages } : {}) });
const ctx = (c: ReturnType<typeof claim>, at = 'happened'): AskContext => {
  const ok = claim('The fix is on the branch.', [TC]);
  const out: AskContext = { happened: [ok], checked: [ok], stakes: ok, recommendation: { option: 'a', why: ok } };
  if (at === 'happened' || at === 'checked') out[at] = [ok, c];
  if (at === 'stakes') out.stakes = c;
  if (at === 'why') out.recommendation = { option: 'a', why: c };
  return out;
};
const s = world();
const A1 = s.runs[RUN_A].agent.id;
const check = (c: ReturnType<typeof claim>, digest = 'jm_task_list {} -> {"title":"212 of 212 checks, 4.2"}', at = 'happened') => ungrounded(ctx(c, at), ev(digest), A1, s.runs);

describe('when a claim is grounded', () => {
  test('a claim must cite something, wherever it sits', () => {
    for (const at of ['happened', 'checked', 'stakes', 'why']) {
      const where = { happened: 'context.happened[1]', checked: 'context.checked[1]', stakes: 'context.stakes', why: 'context.recommendation.why' }[at];
      expect(check(claim('No cites.'), undefined, at)).toBe(`${where} cites nothing. Cite the tool calls or messages it comes from (see evidence).`);
      expect(check(claim('Empty cites.', [], []), undefined, at)?.startsWith(`${where} cites nothing.`)).toBe(true);
    }
  });

  test('every cited id must be a record the adapter found; a tool call must be on a run of the asking agent', () => {
    expect(check(claim('x', [id('tc', 9)]))).toBe('context.happened[1].toolCalls[0] is not a tool call on a run of this agent in this pod.');
    expect(check(claim('x', [TC, TC_B]))).toBe('context.happened[1].toolCalls[1] is not a tool call on a run of this agent in this pod.');
    expect(check(claim('x', undefined, [id('msg', 9)]))).toBe('context.happened[1].messages[0] is not a message in this pod.');
    for (const key of ['__proto__', 'constructor', 'toString']) expect(check(claim('x', [key]))?.includes('is not a tool call')).toBe(true);
    expect(ungrounded(ctx(claim('x', [TC])), undefined, A1, s.runs)?.includes('is not a tool call')).toBe(true);
    expect(ungrounded(ctx(claim('x', undefined, [MSG])), undefined, A1, s.runs)?.includes('context.happened[0].toolCalls[0]')).toBe(true); // no evidence grounds nothing
    expect(ungrounded(ctx(claim('Coverage is 87.5%.', undefined, [MSG])), ev(''), undefined, s.runs)?.includes('context.happened[0].toolCalls[0]')).toBe(true); // a person has no agent
  });

  test('every number must be in a cited digest or a cited message summary or body, as a whole number', () => {
    expect(check(claim('212 of 212 checks passed in 4.2', [TC]))).toBe(null);
    expect(check(claim('212 of 213 checks passed', [TC]))).toBe('Number 2 in context.happened[1] is in none of the evidence it cites. Every number must appear in a cited tool call\'s digest or a cited message\'s summary or body.');
    expect(check(claim('21 checks', [TC]))?.startsWith('Number 1 in context.happened[1]')).toBe(true);
    expect(check(claim('4.20 is out', [TC]))?.startsWith('Number 1 in')).toBe(true);
    expect(check(claim('Coverage is 87.5% across 1024 files, delta 3.', undefined, [MSG]))).toBe(null);
    expect(check(claim('Coverage is 87%.', undefined, [MSG]))?.startsWith('Number 1 in')).toBe(true);
    expect(check(claim('Coverage report 212', undefined, [MSG]))?.startsWith('Number 1 in')).toBe(true); // 212 is in the digest, which this claim does not cite
    expect(check(claim('212 checks, coverage 87.5%', [TC], [MSG]))).toBe(null);
  });

  test('a sign must be one the evidence has; an unsigned number appears in a signed one', () => {
    expect(check(claim('delta -3', undefined, [MSG]))).toBe(null);
    expect(check(claim('delta 3', undefined, [MSG]))).toBe(null);
    expect(check(claim('delta +3', undefined, [MSG]))?.startsWith('Number 1 in')).toBe(true);
    expect(check(claim('gain +212', [TC]))).toBe(null);
    expect(check(claim('loss -212', [TC]))?.startsWith('Number 1 in')).toBe(true);
  });

  test('the answer names the claim and never its text or the evidence', () => {
    const why = check(claim('Secret 31337 rows', [TC]), 'jm_read {} -> token-like-digest 4242') ?? '';
    for (const leak of ['Secret', '31337', 'token-like-digest', '4242', 'jm_read']) expect(why).not.toContain(leak);
  });
});

describe('the post command', () => {
  const ask = (c: ReturnType<typeof claim>): PostInput => ({
    summary: 'Ship?', kind: 'ask', status: 'blocked', needs: { to: { role: 'owner' }, action: 'approve' }, context: ctx(c),
    options: [{ id: 'a', label: 'A', consequence: 'a' }, { id: 'b', label: 'B', consequence: 'b' }],
  } as PostInput);
  const post = (input: PostInput, evidence?: unknown, caller: unknown = { kind: 'run', runId: RUN_A }): Cmd => (
    { type: 'post', caller, messageId: id('msg', 50), decisionId: id('dec', 1), wakeId: id('wak', 1), harness: 'fake', input, evidence } as unknown as Cmd);
  const woken = () => { const w = world(); Object.assign(w.runs[RUN_A], { startedAt: 1, wakeReason: 'task_assigned' }); return w; };

  test('an ungrounded ask is refused ungrounded and nothing is stored; a grounded one is stored', () => {
    const bad = play([[post(ask(claim('Nothing cited.')), ev('d')), 10]], woken());
    expect([errCode(bad.out), events(bad.out).length, bad.state.decisions ?? {}]).toEqual(['ungrounded', 0, {}]);
    const good = play([[post(ask(claim('All 212 passed.', [TC])), ev('212')), 10]], woken());
    expect((events(good.out)[0].data as Message).kind).toBe('ask');
  });

  test('no evidence, or evidence of the wrong shape, grounds nothing; only the pod marker skips the check', () => {
    for (const e of [undefined, null, 'x', 7, [], { toolCalls: 'x', messages: null }]) expect(errCode(play([[post(ask(claim('x', [TC])), e), 10]], woken()).out)).toBe('ungrounded');
    expect(errCode(play([[post(ask(claim('Nothing cited.')), 'pod'), 10]], woken()).out)).toBe(null);
  });

  test('shape problems come first, and a non-ask is never checked', () => {
    expect(errCode(play([[post({ ...ask(claim('x')), receipt: {} } as unknown as PostInput, undefined), 10]], woken()).out)).toBe('invalid_input');
    const fyi = play([[post({ summary: 'done', kind: 'fyi', status: 'done' } as PostInput, undefined), 10]], woken());
    expect(errCode(fyi.out)).toBe(null);
  });

  test('a person may cite a message, not a tool call', () => {
    const owner = { kind: 'user', userId: U_OWNER.id };
    expect(errCode(play([[post(ask(claim('x', [TC])), ev('d'), owner), 10]], woken()).out)).toBe('ungrounded');
    const both = { ...ask(claim('Coverage 87.5%', undefined, [MSG])), context: { happened: [claim('a', undefined, [MSG])], checked: [], stakes: claim('b', undefined, [MSG]), recommendation: { option: 'a', why: claim('c', undefined, [MSG]) } } } as PostInput;
    expect(errCode(play([[post(both, ev('d'), owner), 10]], woken()).out)).toBe(null);
  });
});

describe('the ids an adapter looks up', () => {
  test('every id the claims cite, each once; none for a non-ask or a malformed context', () => {
    const c = ctx(claim('x', [TC, TC_B], [MSG]));
    expect(citedIds({ kind: 'ask', context: c } as PostInput)).toEqual({ toolCalls: [TC, TC_B], messages: [MSG] });
    expect(citedIds({ kind: 'fyi', context: c } as unknown as PostInput)).toEqual({ toolCalls: [], messages: [] });
    expect(citedIds({ kind: 'ask', context: { happened: 'x', stakes: 3, recommendation: null } } as unknown as PostInput)).toEqual({ toolCalls: [], messages: [] });
    expect(citedIds(null as unknown as PostInput)).toEqual({ toolCalls: [], messages: [] });
  });
});
