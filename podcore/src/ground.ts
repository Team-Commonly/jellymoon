// Grounding (schema/message.json x-rules, docs/protocol.md "Rules for Messages and cards"): every claim in an ask cites at least
// one tool call or message, every cited id is a record in this pod (a tool call: on a run of the asking agent), and every number
// in a claim's text appears in what it cites. The core keeps no text (ADR 0003), so the adapter looks the cited ids up and hands
// over what it found; an id it did not find is simply absent, and absent evidence grounds nothing.
import type { AskContext, GroundedClaim, PostInput } from '../../gen/ts/protocol';
import { isObj, own } from './claims';
import type { Evidence, Run } from './types';

/** Every tool-call and message id the claims of an ask cite, each once: what the adapter looks up before the post. */
export function citedIds(input: PostInput): { toolCalls: string[]; messages: string[] } {
  const claims = isObj(input) && input.kind === 'ask' && isObj(input.context) ? claimsOf(input.context).map(([, c]) => c) : [];
  const ids = (k: 'toolCalls' | 'messages'): string[] => [...new Set(claims.flatMap((c) => (Array.isArray(c[k]) ? c[k].filter((x) => typeof x === 'string') : [])))];
  return { toolCalls: ids('toolCalls'), messages: ids('messages') };
}

/** The claims of a context with the place each sits, in a fixed order. A part that is not the schema's shape adds nothing here. */
function claimsOf(ctx: AskContext): [string, GroundedClaim][] {
  const list = (k: 'happened' | 'checked'): [string, GroundedClaim][] => (Array.isArray(ctx[k]) ? ctx[k].map((c, i) => [`context.${k}[${i}]`, c]) : []);
  const one: [string, unknown][] = [['context.stakes', ctx.stakes], ['context.recommendation.why', isObj(ctx.recommendation) ? ctx.recommendation.why : undefined]];
  return [...list('happened'), ...list('checked'), ...one].filter((e): e is [string, GroundedClaim] => isObj(e[1]));
}

/**
 * A number as the rule reads one. Digits (any Unicode decimal digit, kept as written) with single `.` or `,` between digit
 * groups: `4.2` and `1.2.3` are one number each. A comma is a thousands separator only in the `1,234` / `12,345,678.9` shape,
 * and is then dropped; any other comma separates numbers (`1,2` is 1 and 2). A `-`, `+` or `−` directly before the digits is
 * the number's sign unless a letter, digit, `_` or `.` comes before it (`2026-11-09` is 2026, 11 and 09). Digits inside a word
 * still count (`b98213` holds 98213). `%` and other units are not part of the number. Spelled-out numbers are not numbers.
 */
export interface Num { sign: '' | '+' | '-'; digits: string }

const NUM = /((?<![\p{L}\p{N}_.])[+\-−])?(\p{Nd}+(?:[.,]\p{Nd}+)*)/gu;
const THOUSANDS = /^\p{Nd}{1,3}(?:,\p{Nd}{3})+(?:\.\p{Nd}+)?$/u;

export function numbersIn(text: string): Num[] {
  const out: Num[] = [];
  for (const m of text.matchAll(NUM)) {
    const sign = m[1] === undefined ? '' : m[1] === '+' ? '+' : '-';
    const parts = THOUSANDS.test(m[2]) ? [m[2].replaceAll(',', '')] : m[2].split(',');
    parts.forEach((digits, i) => out.push({ sign: i === 0 ? sign : '', digits }));
  }
  return out;
}

/**
 * Whether evidence number `e` grounds claim number `n`: the same digits, and a sign the evidence has. An unsigned claim number
 * appears in a signed one (`3` in `-3`); a `-3` claim needs a `-3`; a `+3` claim takes `+3` or `3`, never `-3`.
 */
const grounds = (e: Num, n: Num): boolean => e.digits === n.digits && (n.sign === '' || e.sign === n.sign || (n.sign === '+' && e.sign === ''));

function texts(claim: GroundedClaim, ev: Evidence | undefined, agentId: string | undefined, runs: Record<string, Run>, at: string): string[] | string {
  const out: string[] = [];
  const tcs = Array.isArray(claim.toolCalls) ? claim.toolCalls : [];
  const msgs = Array.isArray(claim.messages) ? claim.messages : [];
  if (tcs.length + msgs.length === 0) return `${at} cites nothing. Cite the tool calls or messages it comes from (see evidence).`;
  for (const [i, id] of tcs.entries()) {
    const rec = ev !== undefined && isObj(ev.toolCalls) && typeof id === 'string' ? own(ev.toolCalls, id) : undefined;
    const runAgent = rec === undefined ? undefined : own(runs, rec.runId)?.agent.id;
    if (rec === undefined || agentId === undefined || runAgent !== agentId) return `${at}.toolCalls[${i}] is not a tool call on a run of this agent in this pod.`;
    out.push(rec.digest);
  }
  for (const [i, id] of msgs.entries()) {
    const rec = ev !== undefined && isObj(ev.messages) && typeof id === 'string' ? own(ev.messages, id) : undefined;
    if (rec === undefined) return `${at}.messages[${i}] is not a message in this pod.`;
    out.push(rec.summary, rec.body ?? '');
  }
  return out;
}

/**
 * Why an ask's context is not grounded, or null. The answer names the claim by its place (and a citation or number by its
 * index), never by its text, and never echoes evidence. `agentId` is the asking run's agent; a person cites messages only.
 */
export function ungrounded(ctx: AskContext, ev: Evidence | undefined, agentId: string | undefined, runs: Record<string, Run>): string | null {
  for (const [at, claim] of claimsOf(ctx)) {
    const found = texts(claim, ev, agentId, runs, at);
    if (typeof found === 'string') return found;
    const have = found.flatMap((t) => (typeof t === 'string' ? numbersIn(t) : []));
    const missing = numbersIn(typeof claim.text === 'string' ? claim.text : '').findIndex((n) => !have.some((e) => grounds(e, n)));
    if (missing >= 0) return `Number ${missing + 1} in ${at} is in none of the evidence it cites. Every number must appear in a cited tool call's digest or a cited message's summary or body.`;
  }
  return null;
}
