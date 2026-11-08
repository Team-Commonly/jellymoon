// Fixtures and a fake socket for the unit tests. No browser, no network.
import type { Actor, Envelope, EventType, Hello, Message, Option, OptionId } from '../../gen/ts/protocol';
import type { Connect, SocketHandlers } from './store';

export const POD = 'pod_1';
export const you: Actor = { kind: 'human', id: 'usr_you', displayName: 'Ada' };
export const other: Actor = { kind: 'human', id: 'usr_other', displayName: 'Bo' };
const agent: Actor = { kind: 'agent', id: 'agt_1', displayName: 'Scout' };

export function hello(over: Partial<Hello> = {}): Hello {
  return { podId: POD, headSeq: 0, you, members: [{ actor: you, role: 'owner' }, { actor: other, role: 'member' }], agents: [], ...over };
}

export function frame(type: EventType, seq: number | undefined, data: unknown): string {
  const env: Envelope = { v: 1, type, podId: POD, data: data as Record<string, unknown>, ...(seq === undefined ? {} : { seq, ts: '2026-11-08T10:00:00Z' }) };
  return JSON.stringify(env);
}

const opts = (n: number): Option[] => ['a', 'b', 'c', 'd', 'e'].slice(0, n).map((id) => ({ id: id as OptionId, label: `Option ${id}`, consequence: `Does ${id}` }));
const claim = (text: string) => ({ text, toolCalls: ['tc_1'] });

export function ask(seq: number, over: Partial<Message> = {}): Message {
  return {
    id: `msg_${seq}`, seq, podId: POD, author: agent, createdAt: '2026-11-08T09:00:00Z', summary: `Ask ${seq}`, kind: 'ask', status: 'blocked',
    needs: { to: { userIds: [you.id] }, action: 'decide' }, options: opts(3),
    context: { happened: [claim('Tests passed')], checked: [claim('Lint is clean')], stakes: claim('Deploy at noon'), recommendation: { option: 'b', why: claim('Safest') } },
    decision: { id: `dec_${seq}`, state: 'open' }, ...over,
  };
}

export function note(seq: number, over: Partial<Message> = {}): Message {
  return { id: `msg_${seq}`, seq, podId: POD, author: agent, createdAt: '2026-11-08T09:00:00Z', summary: `Note ${seq}`, kind: 'fyi', status: 'done', ...over };
}

/** A connect function whose sockets are recorded; tests push frames in and read frames out. */
export function fakeNet() {
  const live: { h: SocketHandlers; sent: string[]; closed: boolean }[] = [];
  const connect: Connect = (h) => {
    const s = { h, sent: [] as string[], closed: false };
    live.push(s);
    return { send: (t) => void s.sent.push(t), close: () => void (s.closed = true) };
  };
  const last = () => live[live.length - 1];
  return {
    connect, live,
    open: () => last().h.open(),
    deliver: (raw: string) => last().h.message(raw),
    drop: () => last().h.close(),
    sent: () => last().sent.map((t) => JSON.parse(t) as Envelope),
  };
}
