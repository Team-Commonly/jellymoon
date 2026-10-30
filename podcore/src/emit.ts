import type { AnyEvent, ComputerId, ErrorCode, EventDataMap, EventOf, EventType } from '../../gen/ts/protocol';
import { isoOf } from './time';
import type { Ctx, Result } from './types';

/** A members-event: the next per-pod seq, so a log replay has no gaps. */
export function emit<T extends EventType>(c: Ctx, type: T, data: EventDataMap[T]): void {
  c.s.seq += 1;
  const event = { v: 1, type, seq: c.s.seq, podId: c.s.podId, ts: isoOf(c.now), data };
  c.out.push({ kind: 'event', event: event as AnyEvent });
}

/** A to-computer push: no seq, because it is not a members event. */
export function push<T extends 'run.wake' | 'run.stop'>(c: Ctx, computerId: ComputerId, type: T, data: EventDataMap[T]): void {
  c.out.push({ kind: 'to-computer', computerId, event: { v: 1, type, podId: c.s.podId, ts: isoOf(c.now), data } as EventOf<'run.wake'> | EventOf<'run.stop'> });
}

export function ok(c: Ctx, result: Result = null): void {
  c.out.push({ kind: 'reply', ok: true, result });
}

export function fail(c: Ctx, code: ErrorCode, message: string, ref?: string): void {
  c.out.push({ kind: 'reply', ok: false, error: ref === undefined ? { code, message } : { code, message, ref } });
}
