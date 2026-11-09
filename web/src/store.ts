// One store per pod, fed only by the socket. Events apply by seq; anything at or below the last applied seq is
// ignored; a reconnect resubscribes from the last seq. Writes go out as frames and change nothing locally:
// a ruling shows only after the pod's message.updated, and an error frame lands on the card that was waiting.
import type {
  Actor, AgentPresence, Envelope, ErrorBody, Hello, Member, Message, MessageUpdated, OptionId, PresenceChanged, PresenceState, RunEndOutcome, RunEnded, RunStarted, Task, TaskClaimed, ClaimReleased,
} from '../../gen/ts/protocol';

export interface SocketHandlers { open(): void; message(raw: string): void; close(): void }
interface SocketLink { send(text: string): void; close(): void }
export type Connect = (h: SocketHandlers) => SocketLink;

/** A run as the pod's events tell it: going from run.started, ended with its outcome from run.ended. */
export interface RunLine { runId: string; agent: Actor; state: 'going' | 'ended'; outcome?: RunEndOutcome }

interface PodState {
  status: 'connecting' | 'open' | 'reconnecting' | 'stopped';
  you?: Actor;
  members: Member[];
  agents: AgentPresence[];
  presence: Record<string, PresenceState>;
  runs: Record<string, RunLine>;
  headSeq: number;
  lastSeq: number;
  messages: Message[];
  tasks: Record<string, Task>;
  pending: Record<string, OptionId>;
  ruleErrors: Record<string, string>;
  postError?: string;
  droppedFrames: number;
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
  status: 'connecting', members: [], agents: [], presence: {}, runs: {}, headSeq: 0, lastSeq: 0, messages: [], tasks: {}, pending: {}, ruleErrors: {}, droppedFrames: 0,
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
  const { [u.decision.id]: _stale, ...ruleErrors } = s.ruleErrors; // a ruling that landed during a drop clears its "unsent" note
  return { ...s, pending, ruleErrors, messages: s.messages.map((m) => (m.id === u.id ? { ...m, decision: u.decision } : m)) };
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

// One line per going run, and at most one ended line per agent: the agent's latest. A new run for an agent replaces its ended line,
// so an agent is never shown as working and finished at once. Nothing here is timed, so a replay rebuilds exactly the same lines.
const withoutEnded = (runs: Record<string, RunLine>, agentId: string) =>
  Object.fromEntries(Object.entries(runs).filter(([, r]) => !(r.state === 'ended' && r.agent.id === agentId)));
const applyRunStarted = on<RunStarted>((s, r) => {
  if (typeof r.runId !== 'string' || typeof r.agent.id !== 'string') throw new Error('bad run.started');
  return { ...s, runs: { ...withoutEnded(s.runs, r.agent.id), [r.runId]: { runId: r.runId, agent: r.agent, state: 'going' } } };
});
const applyRunEnded = on<RunEnded>((s, r) => {
  if (typeof r.runId !== 'string' || typeof r.agent.id !== 'string' || typeof r.outcome !== 'string') throw new Error('bad run.ended');
  const { [r.runId]: _gone, ...rest } = s.runs;
  return { ...s, runs: { ...withoutEnded(rest, r.agent.id), [r.runId]: { runId: r.runId, agent: r.agent, state: 'ended', outcome: r.outcome } } };
});

const SEQ_EVENTS: Record<string, Apply> = {
  'message.created': applyCreated, 'message.updated': applyUpdated, 'task.created': applyTask, 'task.updated': applyTask,
  'task.claimed': applyClaimed, 'claim.released': applyReleased,
  'run.started': applyRunStarted, 'run.ended': applyRunEnded,
};
// Live events carry no seq and are never replayed on subscribe (podcore/src/presence.ts), so they apply as they come and
// leave lastSeq alone.
const LIVE_EVENTS: Record<string, Apply> = { 'presence.changed': applyPresence };

function applyError(s: PodState, e: ErrorBody): PodState {
  // ref = the ruling's decision id; no ref (an older server) means the most recent waiting ruling; a stale ref changes nothing.
  const waiting = e.ref === undefined ? Object.keys(s.pending).at(-1) : Object.hasOwn(s.pending, e.ref) ? e.ref : undefined;
  if (!waiting) return e.ref === undefined ? { ...s, postError: e.message } : s;
  const { [waiting]: _gone, ...pending } = s.pending;
  return { ...s, pending, ruleErrors: { ...s.ruleErrors, [waiting]: e.message } };
}

/** A dropped socket may have lost a ruling in flight: mark each waiting one unsent so the card can be ruled again. */
function markUnsent(s: PodState): PodState {
  const note = Object.fromEntries(Object.keys(s.pending).map((d) => [d, 'Your ruling may not have reached the pod. Choose again to try again.']));
  return Object.keys(note).length === 0 ? s : { ...s, pending: {}, ruleErrors: { ...s.ruleErrors, ...note } };
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
    try { // a frame that is not JSON, or has no body or a wrong one, is dropped and counted; nothing throws out of here
      const env = JSON.parse(raw) as Envelope;
      if (typeof env.type !== 'string' || typeof env.data !== 'object' || env.data === null) throw new Error('bad frame');
      if (env.type === 'hello') return set(applyHello(state, env.data as never));
      if (env.type === 'error') return set(applyError(state, env.data as unknown as ErrorBody));
      if (env.seq === undefined) return LIVE_EVENTS[env.type] ? set(LIVE_EVENTS[env.type](state, env.data as never)) : undefined;
      if (env.seq <= state.lastSeq) return;
      // A seq'd event this client doesn't know still moves lastSeq, so "caught up with the replay" can't stall on it.
      const apply = SEQ_EVENTS[env.type];
      set({ ...(apply ? apply(state, env.data as never) : state), lastSeq: env.seq });
    } catch { set({ ...state, droppedFrames: state.droppedFrames + 1 }); }
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
        set({ ...markUnsent(state), status: 'reconnecting' });
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
