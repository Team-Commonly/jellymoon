// The pod core's data: plain JSON state, tagged commands, and the ordered output list (ADR 0003).
import type {
  Actor, ActorId, AnyEvent, ComputerId, ErrorBody, EventOf, PodId, RunId, RunOutcome,
  RunStopOutput, Task, TaskClaimOutput, TaskId,
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
}

export interface PodState {
  podId: PodId;
  /** The last event number handed out. */
  seq: number;
  owners: Actor[];
  /** Tasks as stored: the claim lives in `claims`, so a task here never carries one. */
  tasks: Record<TaskId, Task>;
  runs: Record<RunId, Run>;
  /** At most one per task; the key makes exclusivity structural. leaseUntil is in milliseconds. */
  claims: Record<TaskId, { runId: RunId; leaseUntil: number }>;
  /** `${runId}|${taskId}`: how a run lost a claim. Only an expired claim can be restored. */
  lost: Record<string, 'expired' | 'released'>;
}

/** Already authenticated and stamped by the adapter; the core decides what each may do. */
export type Caller = { kind: 'run'; runId: RunId } | { kind: 'user'; userId: ActorId };

export type Cmd =
  | { type: 'claim'; runId: RunId; taskId: TaskId }
  | { type: 'op'; runId: RunId; taskId?: TaskId }
  | { type: 'heartbeat'; computerId: ComputerId }
  | { type: 'task_release'; caller: Caller; taskId: TaskId; reason?: string }
  | { type: 'run_end'; runId: RunId; outcome: RunOutcome }
  | { type: 'run_stop'; caller: Caller; runId: RunId; reason?: string }
  | { type: 'tick' };

export type Result = TaskClaimOutput | Task | RunStopOutput | null;

export type Out =
  | { kind: 'event'; event: AnyEvent }
  | { kind: 'to-computer'; computerId: ComputerId; event: EventOf<'run.stop'> }
  | { kind: 'reply'; ok: true; result: Result }
  | { kind: 'reply'; ok: false; error: ErrorBody };

export interface Ctx { s: PodState; now: number; out: Out[] }
