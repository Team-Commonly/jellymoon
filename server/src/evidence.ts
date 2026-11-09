// Evidence (ADR 0006, "Evidence"): the pod stamps each MCP call a run makes as a tool-call record, and `evidence` returns them.
// A record is made here, from what the adapter itself saw of a call, and by no route: nothing an agent sends reaches it. The
// core only counts (`tool_call`); the record is stored under the count it was given, so the table never holds more than the
// receipt counts, and a call that did not finish (a crash mid-call) leaves a count without a record, never the reverse.
import type { PostInput, ToolCallRecord } from '../../gen/ts/protocol';
import { citedIds } from '../../podcore/src/index';
import type { Evidence } from '../../podcore/src/index';
import type { Hub } from './hub';
import { newId } from './ids';
import { messagesById, saveToolCall, toolCallsById, toolCallsOf } from './store';
import type { ToolCallRow } from './store';

/** At most this many records are kept and returned per run (schema/operations.json: EvidenceOutput). */
const EVIDENCE_CAP = 500;
const NAME_MAX = 64;
const DIGEST_MAX = 600;

/** Closes a call's record once the call has an answer. It never throws: the call has already taken effect. */
export type Finish = (status: 'ok' | 'error', input: unknown, output: string) => Promise<void>;
/** Called before a run's tools/call is dispatched: the pod counts it, and the answer says how to finish its record. Null means the pod could not count it. */
export type Seen = (runId: string, podId: string, name: unknown) => Promise<Finish | null>;

/** The first `max` code points of `s`, with NUL (which Postgres text cannot hold) left out. Walks only as far as it needs: an output can be large. */
export function cut(s: string, max: number): string {
  let out = '';
  let n = 0;
  for (const ch of s) {
    if (ch === '\0') continue;
    if (n++ === max) break;
    out += ch;
  }
  return out;
}

/** The start of the call's input and output as the pod saw them (schema/message.json: ToolCallRecord.digest), cut to 600 characters. */
export function digestOf(name: string, input: unknown, output: string): string {
  return cut(`${name} ${JSON.stringify(input) ?? ''} -> ${output}`, DIGEST_MAX);
}

/** The `seen` hook of the MCP adapter. The count is the core's; the record waits for the call's answer and is saved under that count. */
export function stampCalls(hub: Hub): Seen {
  return async (runId, podId, rawName) => {
    const pod = hub.pods.get(podId);
    if (pod === undefined) return null;
    const name = cut(typeof rawName === 'string' ? rawName : '', NAME_MAX) || '(none)';
    const startedAt = hub.now();
    let before = 0;
    let n = 0;
    const own = (s: { runs: Record<string, { toolCalls?: number }> }): number => (Object.hasOwn(s.runs, runId) ? s.runs[runId].toolCalls ?? 0 : 0);
    const reply = await pod.exec({ type: 'tool_call', runId }, {
      pre: (s) => { before = own(s); return Promise.resolve(null); },
      extra: (_t, s) => { n = own(s) > before ? own(s) : 0; return Promise.resolve(); }, // counted only if the core counted it
    });
    if (reply?.ok !== true) return null;
    return async (status, input, output) => {
      if (n < 1 || n > EVIDENCE_CAP) return;
      const row: ToolCallRow = { id: newId('tc_'), pod: podId, run: runId, n, name, status, startedAt, endedAt: hub.now(), digest: digestOf(name, input, output) };
      await saveToolCall(hub.db, row).catch(() => console.error('a tool call record was not saved', podId));
    };
  };
}

const iso = (ms: number): string => new Date(ms).toISOString();

/** A run's records as ToolCallRecord, in the order the pod counted them. */
export async function readCalls(hub: Hub, podId: string, runId: string): Promise<ToolCallRecord[]> {
  const rows = await toolCallsOf(hub.db, podId, runId, EVIDENCE_CAP);
  return rows.map((r) => ({ id: r.id, runId: r.run, source: 'pod', name: r.name, status: r.status, startedAt: iso(r.startedAt), endedAt: iso(r.endedAt), digest: r.digest }));
}

/** What this pod holds of the ids an ask cites (F25): the core decides from it whether the ask is grounded. Records never change once saved. */
export async function evidenceFor(hub: Hub, podId: string, input: PostInput): Promise<Evidence> {
  const ids = citedIds(input);
  const [calls, msgs] = await Promise.all([toolCallsById(hub.db, podId, ids.toolCalls), messagesById(hub.db, podId, ids.messages)]);
  return {
    toolCalls: Object.fromEntries(calls.map((r) => [r.id, { runId: r.run, digest: r.digest }])),
    messages: Object.fromEntries(msgs.map((m) => [m.id, { summary: m.summary, ...(typeof m.body === 'string' ? { body: m.body } : {}) }])),
  };
}
