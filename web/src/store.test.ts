import { describe, expect, it } from 'vitest';
import { createPodStore } from './store';
import { POD, ask, fakeNet, frame, hello, note, other, you } from './testkit';

function setup() {
  const net = fakeNet();
  const timers: (() => void)[] = [];
  const store = createPodStore({ podId: POD, connect: net.connect, schedule: (fn) => void timers.push(fn) });
  store.start();
  net.open();
  return { net, store, timers };
}
const ids = (s: ReturnType<typeof setup>) => s.store.getState().messages.map((m) => m.id);

describe('subscribe and hello', () => {
  it('subscribes from seq 0 on first open and keeps what hello says', () => {
    const { net, store } = setup();
    expect(net.sent()[0]).toMatchObject({ v: 1, type: 'subscribe', podId: POD, data: { sinceSeq: 0 } });
    net.deliver(frame('hello', undefined, hello({ headSeq: 2 })));
    const s = store.getState();
    expect(s.you).toEqual(you);
    expect(s.members.length).toBe(2);
    expect(s.headSeq).toBe(2);
    expect(s.status).toBe('open');
  });
});

describe('applying events by seq', () => {
  it('applies message.created in order and shows the message', () => {
    const s = setup();
    s.net.deliver(frame('message.created', 1, note(1)));
    s.net.deliver(frame('message.created', 2, note(2)));
    expect(ids(s)).toEqual(['msg_1', 'msg_2']);
    expect(s.store.getState().lastSeq).toBe(2);
  });

  it('ignores a duplicate seq and an out-of-order older seq', () => {
    const s = setup();
    s.net.deliver(frame('message.created', 3, note(3)));
    s.net.deliver(frame('message.created', 3, note(3, { summary: 'changed' })));
    s.net.deliver(frame('message.created', 2, note(2)));
    expect(ids(s)).toEqual(['msg_3']);
    expect(s.store.getState().messages[0].summary).toBe('Note 3');
    expect(s.store.getState().lastSeq).toBe(3);
  });

  it('applies task events, claim release and presence', () => {
    const s = setup();
    const task = { id: 't1', podId: POD, title: 'Fix', kind: 'fix', status: 'open', createdBy: you, createdAt: 'x', updatedAt: 'x' };
    s.net.deliver(frame('task.created', 1, task));
    s.net.deliver(frame('task.claimed', 2, { taskId: 't1', by: you, runId: 'r1', leaseUntil: 'later' }));
    expect(s.store.getState().tasks.t1.claim?.runId).toBe('r1');
    s.net.deliver(frame('claim.released', 3, { taskId: 't1', runId: 'r1', cause: 'released' }));
    expect(s.store.getState().tasks.t1.claim).toBeUndefined();
    s.net.deliver(frame('task.updated', 4, { ...task, status: 'done' }));
    expect(s.store.getState().tasks.t1.status).toBe('done');
    s.net.deliver(frame('presence.changed', 5, { actor: other, state: 'online' }));
    expect(s.store.getState().presence[other.id]).toBe('online');
  });
});

describe('reconnect', () => {
  it('opens a new socket and resubscribes from the last applied seq', () => {
    const s = setup();
    s.net.deliver(frame('message.created', 1, note(1)));
    s.net.deliver(frame('message.created', 2, note(2)));
    s.net.drop();
    expect(s.store.getState().status).toBe('reconnecting');
    expect(s.timers.length).toBe(1);
    s.timers[0]();
    expect(s.net.live.length).toBe(2);
    s.net.open();
    expect(s.net.sent()[0]).toMatchObject({ type: 'subscribe', data: { sinceSeq: 2 } });
    s.net.deliver(frame('message.created', 2, note(2)));
    s.net.deliver(frame('message.created', 3, note(3)));
    expect(ids(s)).toEqual(['msg_1', 'msg_2', 'msg_3']);
  });

  it('does not reconnect after stop', () => {
    const s = setup();
    s.store.stop();
    s.net.drop();
    expect(s.timers.length).toBe(0);
  });
});

describe('writes are never optimistic', () => {
  it('sends decision.rule and shows ruled only after message.updated', () => {
    const s = setup();
    s.net.deliver(frame('message.created', 1, ask(1)));
    s.store.rule('dec_1', 'b');
    expect(s.net.sent().at(-1)).toMatchObject({ v: 1, type: 'decision.rule', podId: POD, data: { decisionId: 'dec_1', optionId: 'b' } });
    const waiting = s.store.getState();
    expect(waiting.messages[0].decision?.state).toBe('open');
    expect(waiting.pending.dec_1).toBe('b');
    s.net.deliver(frame('message.updated', 2, { id: 'msg_1', decision: { id: 'dec_1', state: 'ruled', ruling: { optionId: 'b', by: you, at: 't' } } }));
    const done = s.store.getState();
    expect(done.messages[0].decision?.state).toBe('ruled');
    expect(done.pending.dec_1).toBeUndefined();
  });

  it('attaches an error frame to the card that was waiting', () => {
    const s = setup();
    s.net.deliver(frame('message.created', 1, ask(1)));
    s.store.rule('dec_1', 'a');
    s.net.deliver(frame('error', undefined, { code: 'already_ruled', message: 'Already ruled by Bo' }));
    const st = s.store.getState();
    expect(st.ruleErrors.dec_1).toBe('Already ruled by Bo');
    expect(st.pending.dec_1).toBeUndefined();
    expect(st.messages[0].decision?.state).toBe('open');
  });

  it('sends message.post and does not add the message itself', () => {
    const s = setup();
    s.store.post('hello team', 'msg_9');
    expect(s.net.sent().at(-1)).toMatchObject({ type: 'message.post', data: { text: 'hello team', threadOf: 'msg_9' } });
    expect(s.store.getState().messages.length).toBe(0);
  });

  it('notifies subscribers on change', () => {
    const s = setup();
    let n = 0;
    const off = s.store.subscribe(() => void n++);
    s.net.deliver(frame('message.created', 1, note(1)));
    off();
    s.net.deliver(frame('message.created', 2, note(2)));
    expect(n).toBe(1);
  });
});

describe('errors, bad frames and reconnects', () => {
  const two = (s = setup()) => ([1, 2].forEach((n) => { s.net.deliver(frame('message.created', n, ask(n))); s.store.rule(`dec_${n}`, n === 1 ? 'a' : 'b'); }), s);
  const err = (s: ReturnType<typeof setup>, ref?: string) => s.net.deliver(frame('error', undefined, { code: 'already_ruled', message: 'No', ...(ref && { ref }) }));
  const st = (s: ReturnType<typeof setup>) => s.store.getState();
  it('lands an error on the ruling its ref names, ignores a ref naming nothing waiting, and falls back to the latest with no ref', () => {
    const s = two();
    ['dec_1', 'dec_9', 'constructor'].forEach((ref) => err(s, ref));
    expect([st(s).ruleErrors, st(s).pending]).toEqual([{ dec_1: 'No' }, { dec_2: 'b' }]);
    err(s);
    expect(st(s).ruleErrors).toEqual({ dec_1: 'No', dec_2: 'No' });
  });
  it('drops and counts a frame that is not JSON, or an error, hello or event with no body or a wrong one', () => {
    const s = setup();
    expect(() => {
      ['{not json', 'null'].forEach((raw) => s.net.deliver(raw));
      [['error', 0, undefined], ['error', 0, null], ['hello', 0, null], ['message.updated', 1, null], ['message.updated', 2, {}], ['presence.changed', 3, 'x']]
        .forEach(([type, seq, data]) => s.net.deliver(JSON.stringify({ v: 1, type, podId: POD, ...(seq ? { seq } : {}), data })));
    }).not.toThrow();
    expect([st(s).droppedFrames, st(s).postError]).toEqual([8, undefined]);
  });
  it('re-marks a pending ruling unsent on a drop, clears it if the ruling replays, and lets the person rule again', () => {
    const s = two();
    s.net.drop();
    expect([st(s).pending, st(s).ruleErrors.dec_1]).toEqual([{}, expect.stringMatching(/try again/i)]);
    s.timers[0](); s.net.open();
    s.net.deliver(frame('message.updated', 3, { id: 'msg_1', decision: { id: 'dec_1', state: 'ruled', ruling: { optionId: 'a', by: you, at: 't' } } }));
    s.store.rule('dec_2', 'b');
    expect(st(s).ruleErrors).toEqual({});
  });
});
