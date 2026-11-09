// One store per pod, fed only by the socket. Events apply by seq; anything at or below the last applied seq is
// ignored; a reconnect resubscribes from the last seq. Writes go out as frames and change nothing locally:
// a ruling shows only after the pod's message.updated, and an error frame lands on the card that was waiting.
import type {
  Actor, AgentPresence, Envelope, ErrorBody, Hello, Member, Message, MessageUpdated, OptionId, PresenceChanged, PresenceState, Task, TaskClaimed, ClaimReleased,
} from '../../gen/ts/protocol';

export interface SocketHandlers { open(): void; message(raw: string): void; close(): void }
interface SocketLink { send(text: string): void; close(): void }
export type Connect = (h: SocketHandlers) => SocketLink;

interface PodState {
  status: 'connecting' | 'open' | 'reconnecting' | 'stopped';
  you?: Actor;
  members: Member[];
  agents: AgentPresence[];
  presence: Record<string, PresenceState>;
  headSeq: number;
  lastSeq: number;
  messages: Message[];
  tasks: Record<string, Task>;
  pending: Record<string, OptionId>;
  ruleErrors: Record<string, string>;
  postError?: string;
}

export interface PodStore {
  getState(): PodState;
  subscribe(fn: () => void): () => void;
  start(): void;
  stop(): void;
  rule(decisionId: string, optionId: OptionId): void;
  post(text: string, threadOf?: string): void;
}

const fresh = (): PodState => ({
  status: 'connecting', members: [], agents: [], presence: {}, headSeq: 0, lastSeq: 0, messages: [], tasks: {}, pending: {}, ruleErrors: {},
});

type Apply = (s: PodState, data: never) => PodState;
const on = <T>(f: (s: PodState, d: T) => PodState) => f as unknown as Apply;

const applyHello = on<Hello>((s, d) => ({ ...s, you: d.you, members: d.members, agents: d.agents, headSeq: d.headSeq }));
const applyCreated = on<Message>((s, m) => {
  if (s.messages.some((x) => x.id === m.id)) return s;
  return { ...s, messages: [...s.messages, m].sort((a, b) => a.seq - b.seq) };
});
const applyUpdated = on<MessageUpdated>((s, u) => {
  const { [u.decision.id]: _gone, ...pending } = s.pending;
  return { ...s, pending, messages: s.messages.map((m) => (m.id === u.id ? { ...m, decision: u.decision } : m)) };
});
const applyTask = on<Task>((s, t) => ({ ...s, tasks: { ...s.tasks, [t.id]: t } }));
const applyClaimed = on<TaskClaimed>((s, c) => {
  const t = s.tasks[c.taskId];
  return t ? { ...s, tasks: { ...s.tasks, [t.id]: { ...t, claim: { by: c.by, runId: c.runId, leaseUntil: c.leaseUntil } } } } : s;
});
const applyReleased = on<ClaimReleased>((s, c) => {
  const t = s.tasks[c.taskId];
  if (!t) return s;
  const { claim: _claim, ...rest } = t;
  return { ...s, tasks: { ...s.tasks, [t.id]: rest } };
});
const applyPresence = on<PresenceChanged>((s, p) => ({
  ...s,
  presence: { ...s.presence, [p.actor.id]: p.state },
  agents: s.agents.map((a) => (a.agent.id === p.actor.id ? { ...a, state: p.state } : a)),
}));

const SEQ_EVENTS: Record<string, Apply> = {
  'message.created': applyCreated, 'message.updated': applyUpdated, 'task.created': applyTask, 'task.updated': applyTask,
  'task.claimed': applyClaimed, 'claim.released': applyReleased, 'presence.changed': applyPresence,
};

function applyError(s: PodState, e: ErrorBody): PodState {
  const waiting = Object.keys(s.pending).at(-1);
  if (!waiting) return { ...s, postError: e.message };
  const { [waiting]: _gone, ...pending } = s.pending;
  return { ...s, pending, ruleErrors: { ...s.ruleErrors, [waiting]: e.message } };
}

export function createPodStore(opts: { podId: string; connect: Connect; schedule?: (fn: () => void) => void }): PodStore {
  const schedule = opts.schedule ?? ((fn) => void setTimeout(fn, 1000));
  const listeners = new Set<() => void>();
  let state = fresh();
  let link: SocketLink | null = null;
  const set = (next: PodState) => {
    if (next === state) return;
    state = next;
    listeners.forEach((l) => l());
  };
  const send = (type: string, data: object) => link?.send(JSON.stringify({ v: 1, type, podId: opts.podId, data }));

  function onFrame(raw: string) {
    const env = JSON.parse(raw) as Envelope;
    if (env.type === 'hello') return set(applyHello(state, env.data as never));
    if (env.type === 'error') return set(applyError(state, env.data as unknown as ErrorBody));
    const apply = SEQ_EVENTS[env.type];
    if (!apply || env.seq === undefined || env.seq <= state.lastSeq) return;
    set({ ...apply(state, env.data as never), lastSeq: env.seq });
  }

  function connect() {
    link = opts.connect({
      open: () => {
        set({ ...state, status: 'open' });
        send('subscribe', { sinceSeq: state.lastSeq });
      },
      message: onFrame,
      close: () => {
        if (state.status === 'stopped') return;
        set({ ...state, status: 'reconnecting' });
        schedule(() => state.status === 'reconnecting' && connect());
      },
    });
  }

  return {
    getState: () => state,
    subscribe: (fn) => (listeners.add(fn), () => void listeners.delete(fn)),
    start: connect,
    stop: () => {
      set({ ...state, status: 'stopped' });
      link?.close();
    },
    rule: (decisionId, optionId) => {
      const { [decisionId]: _old, ...ruleErrors } = state.ruleErrors;
      set({ ...state, ruleErrors, pending: { ...state.pending, [decisionId]: optionId } });
      send('decision.rule', { decisionId, optionId });
    },
    post: (text, threadOf) => send('message.post', { text, ...(threadOf && { threadOf }) }),
  };
}
