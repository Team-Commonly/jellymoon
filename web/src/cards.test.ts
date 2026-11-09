import { describe, expect, it } from 'vitest';
import { buildCard, needsYou, shortTime } from './cards';
import { ask, hello, note, other, you } from './testkit';

const noView = {};

describe('buildCard status', () => {
  it('gives a person\'s message no status', () => {
    expect(buildCard(note(1, { author: you, status: 'done' }), noView).status).toBeUndefined();
  });

  it('shows a ruled ask as ruled, not blocked, and an open one as blocked', () => {
    expect(buildCard(ask(1), noView).status).toBe('blocked');
    const ruled = ask(2);
    ruled.decision = { ...ruled.decision!, state: 'ruled', ruling: { optionId: 'b', by: you, at: '2026-11-09T10:43:10.623Z' } };
    expect(buildCard(ruled, noView).status).toBe('ruled');
  });
});

describe('shortTime', () => {
  it('reads as a short local time in the given zone and locale', () => {
    expect(shortTime('2026-11-09T10:43:10.623Z', 'en-US', 'UTC')).toBe('Nov 9, 10:43 AM');
    expect(shortTime('2026-11-09T10:43:10.623Z', 'en-US', 'America/Los_Angeles')).toBe('Nov 9, 2:43 AM');
  });

  it('falls back to the raw text when it is not a time', () => {
    expect(shortTime('soon', 'en-US', 'UTC')).toBe('soon');
  });
});

describe('buildCard', () => {
  it('quiets progress and fyi, and shows headline and status', () => {
    const c = buildCard(note(1, { kind: 'progress', status: 'in_progress', summary: 'Halfway' }), noView);
    expect(c).toMatchObject({ kind: 'progress', quiet: true, headline: 'Halfway', status: 'in_progress', author: 'Scout' });
    expect(buildCard(note(2, { kind: 'result' }), noView).quiet).toBe(false);
  });

  it('folds the body and shows a receipt chip when present', () => {
    const receipt = { runId: 'r', agentId: 'a', computerId: 'c', harness: 'codex' as const, startedAt: 's', postedAt: 'p', toolCalls: 7, wakeReason: 'mention' as const };
    const c = buildCard(note(1, { body: '## detail', receipt }), noView);
    expect(c.body).toBe('## detail');
    expect(c.receipt).toBe('codex · 7 tool calls');
    expect(buildCard(note(3, { receipt: { ...receipt, toolCalls: 1 } }), noView).receipt).toBe('codex · 1 tool call');
    expect(buildCard(note(4, { receipt: { ...receipt, toolCalls: 0 } }), noView).receipt).toBe('codex · 0 tool calls');
    expect(buildCard(note(2), noView).receipt).toBeUndefined();
  });

  it('numbers the options from 1, marks the recommended one and carries the context', () => {
    const c = buildCard(ask(1), noView);
    expect(c.ask?.state).toBe('open');
    expect(c.ask?.options.map((o) => [o.n, o.id, o.recommended])).toEqual([[1, 'a', false], [2, 'b', true], [3, 'c', false]]);
    expect(c.ask?.options[0].consequence).toBe('Does a');
    expect(c.ask?.happened).toEqual(['Tests passed']);
    expect(c.ask?.checked).toEqual(['Lint is clean']);
    expect(c.ask?.nothingChecked).toBe(false);
    expect(c.ask?.stakes).toBe('Deploy at noon');
    expect(c.ask?.recommendation).toEqual({ n: 2, why: 'Safest' });
  });

  it('says nothing was checked when the checked list is empty', () => {
    const m = ask(1);
    const c = buildCard({ ...m, context: { ...m.context!, checked: [] } }, noView);
    expect(c.ask?.nothingChecked).toBe(true);
    expect(c.ask?.checked).toEqual([]);
  });

  it('shows the ruling: option, who and when', () => {
    const c = buildCard(ask(1, { decision: { id: 'dec_1', state: 'ruled', ruling: { optionId: 'c', by: other, at: '2026-11-08T10:00:00Z' } } }), noView);
    expect(c.ask?.state).toBe('ruled');
    expect(c.ask?.ruling).toEqual({ n: 3, optionId: 'c', label: 'Option c', by: 'Bo', at: '2026-11-08T10:00:00Z' });
  });

  it('shows a waiting ruling as pending, not ruled, and an error on the card', () => {
    const waiting = buildCard(ask(1), { pending: 'b' });
    expect(waiting.ask?.state).toBe('open');
    expect(waiting.ask?.ruling).toBeUndefined();
    expect(waiting.ask?.pending).toBe(2);
    expect(buildCard(ask(1), { error: 'Nope' }).ask?.error).toBe('Nope');
  });
});

describe('needsYou', () => {
  const members = hello().members;
  it('keeps open asks that name you by userId', () => {
    const msgs = [ask(1), ask(2, { needs: { to: { userIds: [other.id] }, action: 'decide' } }), note(3)];
    expect(needsYou(msgs, you, members).map((m) => m.id)).toEqual(['msg_1']);
  });

  it('keeps open asks addressed to your role', () => {
    const toOwners = ask(1, { needs: { to: { role: 'owner' }, action: 'approve' } });
    expect(needsYou([toOwners], you, members).length).toBe(1);
    expect(needsYou([toOwners], other, members).length).toBe(0);
  });

  it('drops an ask once it is ruled, so the count falls', () => {
    const open = ask(1);
    const ruled = ask(1, { decision: { id: 'dec_1', state: 'ruled', ruling: { optionId: 'a', by: you, at: 't' } } });
    expect(needsYou([open], you, members).length).toBe(1);
    expect(needsYou([ruled], you, members).length).toBe(0);
  });

  it('ignores asks that name nobody and expired asks', () => {
    const none = ask(1, { needs: undefined });
    const expired = ask(2, { decision: { id: 'dec_2', state: 'expired' } });
    expect(needsYou([none, expired], you, members).length).toBe(0);
  });
});
