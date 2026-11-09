// The boot check: who is signed in (or nobody), decided once, and never stuck on a server that does not answer.
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Actor } from '../../gen/ts/protocol';
import { bootSession, browserHttp, type Http } from './http';

afterEach(() => { vi.useRealTimers(); vi.unstubAllGlobals(); });

const withMe = (session: Http['session']): Http => ({ session, readInvite: async () => null, acceptInvite: async () => { throw new Error('unused'); }, approveDevice: async () => {}, createPod: async () => { throw new Error('unused'); }, stopRun: async () => {}, podAccess: async () => 'member', logout: async () => {} });
const person = { id: 'usr_1' } as unknown as Actor;

describe('bootSession', () => {
  it('is signed out when fetch itself rejects with an AbortError', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => { throw new DOMException('The operation was aborted.', 'AbortError'); }));
    expect(await bootSession(browserHttp, 50)).toBeNull();
  });

  it('asks GET /v1/session, which answers 200 signed out, so a signed-out load logs no failed request', async () => {
    const m = vi.fn(async () => new Response('{}', { status: 200 }));
    vi.stubGlobal('fetch', m);
    expect(await bootSession(browserHttp, 50)).toBeNull();
    expect((m.mock.calls[0] as unknown as [string])[0]).toBe('/v1/session');
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({ user: person }), { status: 200 })));
    expect(await bootSession(browserHttp, 50)).toEqual(person);
  });

  it('is that person when /v1/session names one', async () => {
    expect(await bootSession(withMe(async () => person), 50)).toBe(person);
  });

  it('is signed out when /v1/me says nobody', async () => {
    expect(await bootSession(withMe(async () => null), 50)).toBeNull();
  });

  it('is signed out when the server is unreachable', async () => {
    expect(await bootSession(withMe(async () => { throw new TypeError('fetch failed'); }), 50)).toBeNull();
  });

  it('is signed out, and aborts the request, when the server never answers', async () => {
    vi.useFakeTimers();
    let signal: AbortSignal | undefined;
    const http = withMe((s) => { signal = s; return new Promise(() => {}); });
    let result: Actor | null | undefined;
    void bootSession(http, 5000).then((v) => { result = v; });
    await vi.advanceTimersByTimeAsync(4999);
    expect(result).toBeUndefined();
    await vi.advanceTimersByTimeAsync(1);
    expect(result).toBeNull();
    expect(signal?.aborted).toBe(true);
  });

  it('does not leave a timer running once /v1/me has answered', async () => {
    vi.useFakeTimers();
    await bootSession(withMe(async () => person), 5000);
    expect(vi.getTimerCount()).toBe(0);
  });
});
