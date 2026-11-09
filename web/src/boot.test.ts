// The boot check: signed in or not, decided once, and never stuck on a server that does not answer.
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Actor } from '../../gen/ts/protocol';
import { bootSignedIn, browserHttp, type Http } from './http';

afterEach(() => { vi.useRealTimers(); vi.unstubAllGlobals(); });

const withMe = (session: Http['session']): Http => ({ session, readInvite: async () => null, acceptInvite: async () => { throw new Error('unused'); }, approveDevice: async () => {}, denyDevice: async () => {}, createPod: async () => { throw new Error('unused'); }, stopRun: async () => {}, podAccess: async () => 'member' });
const person = { id: 'usr_1' } as unknown as Actor;

describe('bootSignedIn', () => {
  it('is signed out when fetch itself rejects with an AbortError', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => { throw new DOMException('The operation was aborted.', 'AbortError'); }));
    expect(await bootSignedIn(browserHttp, 50)).toBe(false);
  });

  it('asks GET /v1/session, which answers 200 signed out, so a signed-out load logs no failed request', async () => {
    const m = vi.fn(async () => new Response('{}', { status: 200 }));
    vi.stubGlobal('fetch', m);
    expect(await bootSignedIn(browserHttp, 50)).toBe(false);
    expect((m.mock.calls[0] as unknown as [string])[0]).toBe('/v1/session');
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({ user: person }), { status: 200 })));
    expect(await bootSignedIn(browserHttp, 50)).toBe(true);
  });

  it('is signed in when /v1/session names a person', async () => {
    expect(await bootSignedIn(withMe(async () => person), 50)).toBe(true);
  });

  it('is signed out when /v1/me says nobody', async () => {
    expect(await bootSignedIn(withMe(async () => null), 50)).toBe(false);
  });

  it('is signed out when the server is unreachable', async () => {
    expect(await bootSignedIn(withMe(async () => { throw new TypeError('fetch failed'); }), 50)).toBe(false);
  });

  it('is signed out, and aborts the request, when the server never answers', async () => {
    vi.useFakeTimers();
    let signal: AbortSignal | undefined;
    const http = withMe((s) => { signal = s; return new Promise(() => {}); });
    let result: boolean | undefined;
    void bootSignedIn(http, 5000).then((v) => { result = v; });
    await vi.advanceTimersByTimeAsync(4999);
    expect(result).toBeUndefined();
    await vi.advanceTimersByTimeAsync(1);
    expect(result).toBe(false);
    expect(signal?.aborted).toBe(true);
  });

  it('does not leave a timer running once /v1/me has answered', async () => {
    vi.useFakeTimers();
    await bootSignedIn(withMe(async () => person), 5000);
    expect(vi.getTimerCount()).toBe(0);
  });
});
