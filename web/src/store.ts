// Stub: the tests are committed red first; the real store lands in the next commit.
import type { Actor, AgentPresence, Member, Message, OptionId, PresenceState, Task } from '../../gen/ts/protocol';

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

export function createPodStore(_opts: { podId: string; connect: Connect; schedule?: (fn: () => void) => void }): PodStore {
  throw new Error('not implemented');
}
