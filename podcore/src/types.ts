// The pod core's data: plain JSON state, tagged commands, and the ordered output list (ADR 0003).
import type {
  Actor, ActorId, AgentId, AnyEvent, ClaimRunResponse, ComputerId, ErrorBody, EventOf, Message, MessageId, PodId, RunId, RunOutcome,
  RunStopOutput, RunWake, Task, TaskClaimOutput, TaskId, TaskKind, TaskUpdateStatus, WakeId, WakeReason, WakeRefs,
} from '../../gen/ts/protocol';

export interface Run {
  id: RunId;
  /** The agent this run belongs to. */
  agent: Actor;
  /** The person who owns that agent (the user whose computer registered it). */
  owner: Actor;
  computerId: ComputerId;
  /** Milliseconds; the 2 h token cap counts from here. */
  startedAt: number;
  status: 'active' | 'ended';
  outcome?: RunOutcome;
  /** Set when a person stopped the run; cleared when the daemon reports it ended. Re-sent on each heartbeat until then. */
  stop?: { reason?: string };
}

/** Who hosts an agent and who owns it; the adapter registers it (ADR 0003). */
export interface Agent { actor: Actor; owner: Actor; computerId: ComputerId }

/** A pending wake. Keyed by agent in PodState, so a second pending wake for one agent cannot exist. */
export interface Wake { id: WakeId; agentId: AgentId; reason: WakeReason; refs: WakeRefs; sentAt: number }

export interface PodState {
  podId: PodId;
  /** The last event number handed out. */
  seq: number;
  /** Every person in the pod, owners included. Only a member may call as a user. Empty until `pod_create`. */
  members: Actor[];
  owners: Actor[];
  /** Tasks as stored: the claim lives in `claims`, so a task here never carries one. */
  tasks: Record<TaskId, Task>;
  runs: Record<RunId, Run>;
  /** At most one per task; the key makes exclusivity structural. leaseUntil is in milliseconds. */
  claims: Record<TaskId, { runId: RunId; leaseUntil: number }>;
  /** How a run lost a claim, by run then task. Only an expired claim can be restored; a person's release cannot. */
  lost: Record<RunId, Record<TaskId, 'expired' | 'released'>>;
  agents: Record<AgentId, Agent>;
  wakes: Record<AgentId, Wake>;
}

/** Already authenticated and stamped by the adapter; the core decides what each may do. */
export type Caller = { kind: 'run'; runId: RunId } | { kind: 'user'; userId: ActorId };

export type Cmd =
  | { type: 'claim'; runId: RunId; taskId: TaskId }
  | { type: 'op'; runId: RunId; taskId?: TaskId }
  | { type: 'heartbeat'; computerId: ComputerId }
  | { type: 'task_release'; caller: Caller; taskId: TaskId; reason?: string }
  | { type: 'run_end'; runId: RunId; computerId: ComputerId; outcome: RunOutcome }
  | { type: 'run_stop'; caller: Caller; runId: RunId; reason?: string }
  | { type: 'pod_create'; ownerId: ActorId; ownerName: string }
  | { type: 'member_add'; userId: ActorId; name: string }
  | { type: 'task_create'; caller: Caller; taskId: TaskId; title: string; kind: TaskKind; assignee?: ActorId; body?: string; wakeId: WakeId }
  | { type: 'task_update'; caller: Caller; taskId: TaskId; status?: TaskUpdateStatus; assignee?: ActorId; wakeId: WakeId }
  | { type: 'message_post'; caller: Caller; messageId: MessageId; text: string; threadOf?: MessageId; wakeId: WakeId }
  | { type: 'boot'; staleMs: number }
  | { type: 'change'; kind: 'ruling'; agentId: AgentId; refs: WakeRefs; wakeId: WakeId }
  | { type: 'wake_ack'; wakeId: WakeId; computerId: ComputerId }
  | { type: 'run_claim'; wakeId: WakeId; computerId: ComputerId; runId: RunId }
  | { type: 'agent_register'; agentId: AgentId; ownerId: ActorId; computerId: ComputerId; agentName?: string; ownerName?: string }
  | { type: 'tick' };

export type RunClaimed = Pick<ClaimRunResponse, 'runId' | 'expiresAt'> & Omit<RunWake, 'wakeId' | 'agentId'>;
export type Result = TaskClaimOutput | Task | RunStopOutput | RunClaimed | Message | null;

export type Out =
  | { kind: 'event'; event: AnyEvent }
  | { kind: 'to-computer'; computerId: ComputerId; event: EventOf<'run.stop'> | EventOf<'run.wake'> }
  | { kind: 'reply'; ok: true; result: Result }
  | { kind: 'reply'; ok: false; error: ErrorBody };

/** `opened` counts the new wakes this command has written; the k-th takes `nthWakeId(seed, k)`. */
export interface Ctx { s: PodState; now: number; out: Out[]; opened: number }
