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

/**
 * `v` with every occurrence of `secret` replaced, in raw and in JSON-escaped form: the one choke point every upstream answer
 * and error passes through before an agent, a person, a receipt or a log sees it.
 */
export function scrub<T>(v: T, secret: string): T {
  const text = JSON.stringify(v);
  if (text === undefined || secret === '') return v;
  const escaped = JSON.stringify(secret).slice(1, -1);
  // A secret made of JSON punctuation could straddle the structure; then nothing of the value is kept.
  try { return JSON.parse(text.split(secret).join(REMOVED).split(escaped).join(REMOVED)) as T; } catch { return REMOVED as T; }
}
