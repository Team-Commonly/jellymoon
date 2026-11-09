// What the browser specs drive the pod with: the real routes of the server global-setup started, never a fake pod.
// A person is minted by POST /__test/session (ENV=local). The fake agent is a computer and an agent registered through the
// real routes; it gets a run token the way the daemon does (a task wakes it, it acks the wake, POST /v1/runs) and posts with it.
import WebSocket from 'ws';
import type { Actor, Envelope, PostInput } from '../gen/ts/protocol';

const base = (): string => {
  const b = process.env.JM_WEB_E2E_BASE;
  if (!b) throw new Error('JM_WEB_E2E_BASE is unset: run the specs through web-e2e/playwright.config.ts');
  return b;
};
const wsUrl = (podId: string): string => `${base().replace(/^http/, 'ws')}/v1/pods/${podId}/ws`;

export async function api<T>(method: string, path: string, token: string | null, body?: unknown): Promise<T> {
  const headers: Record<string, string> = { 'content-type': 'application/json', ...(token ? { authorization: `Bearer ${token}` } : {}) };
  const res = await fetch(base() + path, { method, headers, ...(body === undefined ? {} : { body: JSON.stringify(body) }) });
  const text = await res.text();
  if (!res.ok) throw new Error(`${method} ${path}: HTTP ${res.status} ${text}`);
  return (text ? JSON.parse(text) : {}) as T;
}

/** A Node-side pod socket with a bearer token (a browser cannot set one). Collects every frame it receives. */
class PodSocket {
  readonly frames: Envelope[] = [];
  private waiters: Array<() => void> = [];
  private constructor(private ws: WebSocket, readonly podId: string) {
    ws.on('message', (d: unknown) => {
      this.frames.push(JSON.parse(String(d)) as Envelope);
      this.waiters.splice(0).forEach((w) => w());
    });
  }

  static open(podId: string, token: string): Promise<PodSocket> {
    const ws = new WebSocket(wsUrl(podId), { headers: { authorization: `Bearer ${token}` } });
    return new Promise((done, fail) => {
      ws.once('open', () => done(new PodSocket(ws, podId)));
      ws.once('error', fail);
    });
  }

  send(type: string, data: object): void {
    this.ws.send(JSON.stringify({ v: 1, type, podId: this.podId, data }));
  }

  /** The first frame matching `ok`, waiting up to 10 s for it. */
  async next(what: string, ok: (f: Envelope) => boolean): Promise<Envelope> {
    for (const until = Date.now() + 10_000; ;) {
      const hit = this.frames.find(ok);
      if (hit) return hit;
      const left = until - Date.now();
      if (left <= 0) throw new Error(`no ${what} within 10 s; got ${this.frames.map((f) => f.type).join(', ')}`);
      await new Promise<void>((r) => {
        const t = setTimeout(r, left);
        this.waiters.push(() => { clearTimeout(t); r(); });
      });
    }
  }

  close(): void {
    this.ws.close();
  }
}

/** Every event the pod holds, from a fresh subscribe at seq 0: what the API says happened, independent of any page. */
async function replay(podId: string, token: string): Promise<Envelope[]> {
  const s = await PodSocket.open(podId, token);
  try {
    const hello = await s.next('hello', (f) => f.type === 'hello');
    const head = (hello.data as { headSeq: number }).headSeq;
    s.send('subscribe', { sinceSeq: 0 });
    if (head > 0) await s.next(`replay to seq ${head}`, (f) => f.seq === head);
    return s.frames.filter((f) => f.seq !== undefined);
  } finally {
    s.close();
  }
}

/** The decision's state as the pod's own log has it: the last message.updated for it, or open. */
export async function decisionOf(podId: string, token: string, decisionId: string): Promise<{ state: string; optionId?: string; by?: string }> {
  const ups = (await replay(podId, token)).filter((f) => f.type === 'message.updated' && (f.data as { decision: { id: string } }).decision.id === decisionId);
  const last = ups.at(-1)?.data as { decision: { state: string; ruling?: { optionId: string; by: Actor } } } | undefined;
  return last ? { state: last.decision.state, optionId: last.decision.ruling?.optionId, by: last.decision.ruling?.by.id } : { state: 'open' };
}

/** How the pod's own log says the run ended (its run.ended outcome), or null while the run is going. */
export async function runOutcomeOf(podId: string, token: string, runId: string): Promise<string | null> {
  const ends = (await replay(podId, token)).filter((f) => f.type === 'run.ended' && (f.data as { runId: string }).runId === runId);
  return ends.length === 0 ? null : (ends.at(-1)!.data as { outcome: string }).outcome;
}

export interface Person { token: string; user: Actor }
export const person = (name: string): Promise<Person> => api<Person>('POST', '/__test/session', null, { name });

/** The fake agent: one computer of the owner's, one agent on it, holding a live run token. */
/** `cited` is a tool call the pod stamped on the agent's run (one MCP call at start), which its asks cite (F25: every claim cites a record). */
export interface FakeAgent { agentId: string; runId: string; cited: string; post(input: PostInput): Promise<{ messageId: string; decisionId?: string; seq: number }>; close(): void }

export async function fakeAgent(podId: string, owner: Person, name: string): Promise<FakeAgent> {
  const comp = await api<{ token: string }>('POST', '/v1/computers', owner.token, { name: 'browser-spec-box', os: 'linux', arch: 'amd64' });
  const { agentId } = await api<{ agentId: string }>('POST', `/v1/pods/${podId}/agents`, comp.token, { harness: 'fake', displayName: name });
  const computer = await PodSocket.open(podId, comp.token);
  const board = await PodSocket.open(podId, owner.token);
  try {
    await computer.next('hello on the computer socket', (f) => f.type === 'hello');
    await board.next('hello on the owner socket', (f) => f.type === 'hello');
    board.send('task.create', { title: 'Look at the auth fix', kind: 'chore', assignee: agentId });
    const wake = await computer.next('run.wake for the agent', (f) => f.type === 'run.wake' && (f.data as { agentId: string }).agentId === agentId);
    const wakeId = (wake.data as { wakeId: string }).wakeId;
    computer.send('wake.ack', { wakeId });
    const { runId, runToken } = await api<{ runId: string; runToken: string }>('POST', '/v1/runs', comp.token, { wakeId });
    await api('POST', '/mcp', runToken, { jsonrpc: '2.0', id: 1, method: 'tools/call', params: { name: 'jm_task_list', arguments: {} } });
    const { toolCalls } = await api<{ toolCalls: Array<{ id: string }> }>('GET', `/v1/pods/${podId}/runs/${runId}/tool-calls`, runToken);
    return {
      agentId,
      runId,
      cited: toolCalls[0].id,
      post: (input) => api('POST', `/v1/pods/${podId}/messages`, runToken, input),
      close: () => computer.close(),
    };
  } finally {
    board.close();
  }
}

/** An ask as an agent writes it: two options, the first recommended, addressed by `to`, every claim citing the tool call `cited`. */
export function askInput(summary: string, to: { userIds: string[] } | { role: string }, cited: string): PostInput {
  const claim = { text: 'The fix passed every test.', toolCalls: [cited] };
  return {
    summary, kind: 'ask', status: 'blocked',
    needs: { to, action: 'approve' },
    options: [
      { id: 'a', label: 'Merge now', consequence: 'The agent merges on its next run.' },
      { id: 'b', label: 'Wait', consequence: 'The agent waits for review.' },
    ],
    context: { happened: [claim], checked: [], stakes: { text: 'Waiting holds the release a day.', toolCalls: claim.toolCalls }, recommendation: { option: 'a', why: claim } },
  } as PostInput;
}
