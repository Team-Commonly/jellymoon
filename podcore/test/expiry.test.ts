import { describe, expect, test } from 'bun:test';
import type { AskContext, PostInput } from '../../gen/ts/protocol';
import { apply, nextDeadline } from '../src';
import type { Cmd, PodState } from '../src';
import { RUN_A, U_OWNER, errCode, events, id, play, toComputer, wak, world } from './helpers';

// An ask the adapter gave an expiry (a connector write's ask, ADR 0009): open until then, expired after, never ruled late.
const DEC = id('dec', 1);
const MSG = id('msg', 1);
const claim = { text: 'The agent asked for one write.' };
const ctx: AskContext = { happened: [claim], checked: [], stakes: claim, recommendation: { option: 'b', why: claim } };
const ask: PostInput = {
  summary: 'Approve linear create_issue?', kind: 'ask', status: 'blocked', needs: { to: { role: 'owner' }, action: 'approve' }, context: ctx,
  options: [{ id: 'a', label: 'Approve', consequence: 'One call is made.' }, { id: 'b', label: 'Reject', consequence: 'Nothing is sent.' }],
};
const post = (expiresAt: unknown): Cmd => ({ type: 'post', caller: { kind: 'run', runId: RUN_A }, messageId: MSG, decisionId: DEC, wakeId: wak(1), harness: 'fake', input: ask, expiresAt, evidence: 'pod' } as unknown as Cmd); // the connector write card: the pod composes it
const rule = (): Cmd => ({ type: 'rule', caller: { kind: 'user', userId: U_OWNER.id }, decisionId: DEC, option: 'a', wakeId: wak(2) } as unknown as Cmd);

const parked = (expiresAt: unknown = 50_000): PodState => {
  const s = world();
  Object.assign(s.runs[RUN_A], { startedAt: 1000, wakeReason: 'task_assigned' });
  return play([[post(expiresAt), 2000]], s).state;
};

describe('an ask with an expiry', () => {
  test('its expiry is a deadline, and a tick at it expires the decision and tells members', () => {
    const s = parked();
    expect(nextDeadline(s)! <= 50_000).toBe(true);
    const t = apply(s, { type: 'tick' }, 50_000);
    const up = events(t.out).filter((e) => e.type === 'message.updated').map((e) => e.data);
    expect(up).toEqual([{ id: MSG, decision: { id: DEC, state: 'expired' } }]);
    expect(t.state.decisions?.[DEC]).toEqual({ messageId: MSG, askedBy: s.decisions?.[DEC].askedBy, options: [], decision: { id: DEC, state: 'expired' } });
  });

  test('a ruling after the expiry gets gone, with no event and no wake; one before it rules as usual', () => {
    const late = apply(parked(), rule(), 50_000);
    expect([errCode(late.out), toComputer(late.out).length, events(late.out).filter((e) => e.type !== 'message.updated').length]).toEqual(['gone', 0, 0]);
    expect(late.state.decisions?.[DEC].decision.state).toBe('expired');
    const early = apply(parked(), rule(), 49_999);
    expect([errCode(early.out), early.state.decisions?.[DEC].decision.state]).toEqual([null, 'ruled']);
  });

  test('an expiry that is not a later integer time is refused before anything is stored', () => {
    for (const bad of [2000, 1500, 'soon', 2000.5]) {
      const s = world();
      Object.assign(s.runs[RUN_A], { startedAt: 1000, wakeReason: 'task_assigned' });
      const r = apply(s, post(bad), 2000);
      expect([bad, errCode(r.out), r.state.decisions]).toEqual([bad, 'invalid_input', undefined]);
    }
  });
});
