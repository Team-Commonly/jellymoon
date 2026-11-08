import { expect, test } from 'bun:test';
import { PodDO } from '../src/pod-do';
import { hello } from '../src/views';
import { encodeWho, WHO_HEADER } from '../src/wire';
import type { Who } from '../src/wire';
import type { Env } from '../src/env';

// presence.changed has no seq and is replayed to nobody, so a client that is not yet subscribed misses it. The pod object runs
// here over a fake of its storage and sockets; the cases below drive its real handlers.
const OWNER = 'usr_01ARZ3NDEKTSV4RRFFQ69G5FAV';
const AGENT = 'agt_01ARZ3NDEKTSV4RRFFQ69G5FAW';
const COMPUTER = 'cmp_01ARZ3NDEKTSV4RRFFQ69G5FAX';
const OWNER_ACTOR = { kind: 'human', id: OWNER, displayName: 'Owner' } as const;
const user: Who = { kind: 'user', actor: OWNER_ACTOR };
const computer: Who = { kind: 'computer', computerId: COMPUTER, owner: OWNER_ACTOR };

class FakeSocket {
  sent: string[] = [];
  readyState = 1;
  private att: unknown = null;
  send(t: string): void { this.sent.push(t); }
  serializeAttachment(v: unknown): void { this.att = v; }
  deserializeAttachment(): unknown { return this.att; }
  frames(type: string): Array<{ data: Record<string, unknown> }> { return this.sent.map((t) => JSON.parse(t)).filter((f) => f.type === type); }
}

/** The few statements the pod object runs, kept in memory. */
function fakeSql() {
  const kv = new Map<string, string>();
  const events: Array<[number, string]> = [];
  return {
    exec(q: string, ...a: unknown[]) {
      let rows: unknown[] = [];
      if (q.startsWith('insert or replace into kv')) kv.set(a[0] as string, a[1] as string);
      else if (q.startsWith('insert into events')) events.push([a[0] as number, a[1] as string]);
      else if (q.startsWith('select envelope')) rows = events.filter(([s]) => s > (a[0] as number)).slice(0, a[1] as number).map(([, envelope]) => ({ envelope }));
      return { toArray: () => rows, rowsWritten: 1 };
    },
  };
}

function podObject() {
  const sockets: Array<{ ws: FakeSocket; tags: string[] }> = [];
  const ctx = {
    storage: { sql: fakeSql(), transactionSync: (f: () => void) => f(), setAlarm: async () => {}, deleteAlarm: async () => {} },
    blockConcurrencyWhile: (f: () => void) => f(),
    acceptWebSocket: (ws: FakeSocket, tags: string[]) => { sockets.push({ ws, tags }); },
    getWebSockets: (tag?: string) => sockets.filter((s) => tag === undefined || s.tags.includes(tag)).map((s) => s.ws),
    getTags: (ws: FakeSocket) => sockets.find((s) => s.ws === ws)?.tags ?? [],
  };
  (globalThis as unknown as { WebSocketPair: unknown }).WebSocketPair = class { 0 = {}; 1 = new FakeSocket(); };
  const pod = new PodDO(ctx as unknown as DurableObjectState, {} as Env);
  pod.begin('pod_1');
  pod.exec({ type: 'pod_create', ownerId: OWNER, ownerName: 'Owner' });
  pod.exec({ type: 'agent_register', agentId: AGENT, ownerId: OWNER, computerId: COMPUTER });
  return { pod, sockets };
}

async function connect(pod: PodDO, who: Who): Promise<void> {
  await pod.fetch(new Request('https://pod/v1/pods/pod_1/ws', { headers: { upgrade: 'websocket', [WHO_HEADER]: encodeWho(who) } }));
}

const agentState = (f: { data: Record<string, unknown> } | undefined): unknown => (f?.data.agents as Array<{ state: string }> | undefined)?.[0]?.state;

test('presence that changes between hello and subscribe reaches the client in a fresh hello', async () => {
  const { pod, sockets } = podObject();
  await connect(pod, user);
  const client = sockets[0].ws;
  expect(agentState(client.frames('hello')[0])).toBe('offline');
  await connect(pod, computer); // the agent's computer comes online: presence.changed, sent to no one yet
  expect(client.frames('presence.changed').length).toBe(0);
  expect(agentState({ data: { ...hello(pod.state, pod.meta, OWNER_ACTOR) } })).toBe('online'); // the pod itself knows
  pod.webSocketMessage(client as unknown as WebSocket, JSON.stringify({ v: 1, type: 'subscribe', podId: 'pod_1', data: { sinceSeq: 0 } }));
  expect(client.frames('error').length).toBe(0);
  const last = client.frames('hello').at(-1);
  expect(agentState(last)).toBe('online');
});

// A refused ruling names the decision it was about, so the web store can put the error on that card.
const sendFrame = async (type: string, data: unknown): Promise<Array<{ data: Record<string, unknown> }>> => {
  const { pod, sockets } = podObject();
  await connect(pod, user);
  pod.webSocketMessage(sockets[0].ws as unknown as WebSocket, JSON.stringify({ v: 1, type, podId: 'pod_1', data }));
  return sockets[0].ws.frames('error');
};

test('a ruling the core refuses comes back as an error frame whose ref is the decision id', async () => {
  const decisionId = 'dec_01ARZ3NDEKTSV4RRFFQ69G5FAZ';
  expect((await sendFrame('decision.rule', { decisionId, optionId: 'a' })).map((f) => f.data)).toEqual([{ code: 'not_found', message: 'No such decision.', ref: decisionId }]);
});

test('an error frame that is not about a ruling carries no ref', async () => {
  expect((await sendFrame('message.post', {})).map((f) => Object.keys(f.data))).toEqual([['code', 'message']]);
});
