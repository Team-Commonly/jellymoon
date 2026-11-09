// The catalogue pins and the protocol-neutral pieces of the connector layer (ADR 0009): a pinned operation is (vendor, name,
// class, argument names), and everything keys on (connection, operation, argsHash). Nothing here knows what protocol an
// upstream speaks; that is the adapter's file (upstream-mcp.ts).
import { createHash } from 'node:crypto';
import linear from '../connectors/linear.json';
import type { ConnectorVendor, OperationClass, PinnedOperation } from '../../gen/ts/protocol';

export interface PinOp { name: string; class: OperationClass; args: string[]; required: string[] }
export interface Pin { vendor: ConnectorVendor; protocol: 'mcp-http'; operations: PinOp[] }
/** An operation as an upstream lists it, reduced to what the pin compares. */
export interface ListedOp { name: string; args: string[]; required: string[] }

const PINS: Record<ConnectorVendor, Pin> = { linear: linear as Pin };

export const pinOf = (vendor: ConnectorVendor): Pin => PINS[vendor];
export const opOf = (pin: Pin, name: string): PinOp | undefined => pin.operations.find((o) => o.name === name);
export const view = (o: PinOp): PinnedOperation => ({ name: o.name, class: o.class, args: [...o.args] });

const same = (a: string[], b: string[]): boolean => a.length === b.length && [...a].sort().every((x, i) => x === [...b].sort()[i]);

/** Why the upstream's list does not match the pin, naming the operation, or null when every pinned operation is there as pinned. */
export function driftOf(pin: Pin, listed: ListedOp[]): string | null {
  if (listed.length === 0) return `The upstream lists no operations; the ${pin.vendor} pin needs ${pin.operations.map((o) => o.name).join(', ')}.`;
  for (const p of pin.operations) {
    const up = listed.find((l) => l.name === p.name);
    if (up === undefined) return `The upstream does not list ${p.name}, which the ${pin.vendor} pin names.`;
    if (!same(up.args, p.args) || !same(up.required, p.required)) {
      return `${p.name} changed upstream: it takes ${up.args.join(', ') || 'no arguments'} (required: ${up.required.join(', ') || 'none'}); the pin has ${p.args.join(', ')} (required: ${p.required.join(', ') || 'none'}).`;
    }
  }
  return null;
}

/** JSON with every object's keys sorted: the form argsHash is taken over. */
export function canonical(v: unknown): string {
  if (Array.isArray(v)) return `[${v.map(canonical).join(',')}]`;
  if (v !== null && typeof v === 'object') {
    const o = v as Record<string, unknown>;
    return `{${Object.keys(o).sort().map((k) => `${JSON.stringify(k)}:${canonical(o[k])}`).join(',')}}`;
  }
  return JSON.stringify(v) ?? 'null';
}

export const argsHash = (args: unknown): string => createHash('sha256').update(canonical(args)).digest('hex');

const REMOVED = '[credential removed]';

const hexChar = (_: string, h: string): string => String.fromCharCode(parseInt(h, 16));

/** Whether `s` holds `secret` once decoded: \u escapes, percent-encoding (with + as a space or not), or base64 (std or url). */
function encodes(s: string, secret: string): boolean {
  const views = [s.replace(/\\u([0-9a-fA-F]{4})/g, hexChar).replace(/\\(["\\/])/g, '$1'), s.replace(/%([0-9a-fA-F]{2})/g, hexChar), s.replace(/\+/g, ' ').replace(/%([0-9a-fA-F]{2})/g, hexChar)];
  if (views.some((x) => x.includes(secret))) return true;
  for (const run of s.match(/[A-Za-z0-9+/_-]{12,}/g) ?? []) {
    const std = run.replace(/-/g, '+').replace(/_/g, '/');
    for (let at = 0; at < 4; at++) {
      const tail = std.slice(at);
      const part = tail.length % 4 === 1 ? tail.slice(0, -1) : tail + '='.repeat((4 - (tail.length % 4)) % 4);
      try { if (atob(part).includes(secret)) return true; } catch { /* not base64 at this offset */ }
    }
  }
  return false;
}

/** Each string (and key) of a JSON value that holds `secret` in an encoded form becomes REMOVED, whole. */
function dropEncoded(v: unknown, secret: string): unknown {
  if (typeof v === 'string') return encodes(v, secret) ? REMOVED : v;
  if (Array.isArray(v)) return v.map((x) => dropEncoded(x, secret));
  if (v === null || typeof v !== 'object') return v;
  return Object.fromEntries(Object.entries(v).map(([k, x]) => [encodes(k, secret) ? REMOVED : k, dropEncoded(x, secret)]));
}

/**
 * `v` with `secret` removed: every raw or JSON-escaped occurrence replaced, then any string that still holds it \u-escaped,
 * percent-encoded or base64-encoded dropped whole. The one choke point every upstream answer and error passes through before
 * an agent, a person, a receipt or a log sees it.
 */
export function scrub<T>(v: T, secret: string): T {
  const text = JSON.stringify(v);
  if (text === undefined || secret === '') return v;
  const escaped = JSON.stringify(secret).slice(1, -1);
  // A secret made of JSON punctuation could straddle the structure; then nothing of the value is kept.
  try { return dropEncoded(JSON.parse(text.split(secret).join(REMOVED).split(escaped).join(REMOVED)), secret) as T; } catch { return REMOVED as T; }
}
