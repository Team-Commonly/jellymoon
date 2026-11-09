// The browser client's request shape. The server refuses a cookie write that is not JSON (403), so every write sends it.
import { afterEach, describe, expect, it, vi } from 'vitest';
import { browserHttp } from './http';

afterEach(() => vi.unstubAllGlobals());

function stub(body: unknown = {}) {
  const fetchMock = vi.fn(async () => new Response(JSON.stringify(body), { status: 200 }));
  vi.stubGlobal('fetch', fetchMock);
  return fetchMock;
}
const sent = (m: ReturnType<typeof stub>) => {
  const [url, init] = m.mock.calls[0] as unknown as [string, RequestInit];
  return { url, method: init.method, type: new Headers(init.headers).get('content-type'), body: init.body, credentials: init.credentials };
};

describe('browserHttp cookie writes', () => {
  it('acceptInvite: POST, same-origin cookie, a JSON body, the code only in the path', async () => {
    const m = stub({ pod: { id: 'pod_9' } });
    await browserHttp.acceptInvite('a b/c');
    expect(sent(m)).toEqual({ url: '/v1/invites/a%20b%2Fc/accept', method: 'POST', type: 'application/json', body: '{}', credentials: 'same-origin' });
  });

  it('approveDevice: POST, same-origin cookie, the user code as a JSON body', async () => {
    const m = stub();
    await browserHttp.approveDevice('WDJB-MJHT');
    expect(sent(m)).toEqual({ url: '/v1/auth/device/approve', method: 'POST', type: 'application/json', body: '{"userCode":"WDJB-MJHT"}', credentials: 'same-origin' });
  });

  it('approveDevice: a refusal carries the status and the server\'s error code', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({ code: 'invalid_input', message: 'does not match the required pattern' }), { status: 400 })));
    await expect(browserHttp.approveDevice('ABCD-EFGH')).rejects.toMatchObject({ status: 400, code: 'invalid_input' });
    vi.stubGlobal('fetch', vi.fn(async () => new Response('<html>', { status: 502 })));
    await expect(browserHttp.approveDevice('WDJB-MJHT')).rejects.toMatchObject({ status: 502, code: undefined });
  });

  it('session: a read with the cookie; no user is nobody', async () => {
    const m = stub({ user: { kind: 'human', id: 'usr_1', displayName: 'Ada' } });
    expect(await browserHttp.session()).toMatchObject({ id: 'usr_1' });
    expect(sent(m)).toMatchObject({ url: '/v1/session', method: undefined, credentials: 'same-origin' });
    stub({});
    expect(await browserHttp.session()).toBeNull();
  });

  it('session: hands the caller\'s abort signal to fetch', async () => {
    const m = stub({ user: { kind: 'human', id: 'usr_1', displayName: 'Ada' } });
    const ctl = new AbortController();
    await browserHttp.session(ctl.signal);
    expect((m.mock.calls[0] as unknown as [string, RequestInit])[1].signal).toBe(ctl.signal);
  });
});

describe('browserHttp readInvite', () => {
  it('GETs the invite with no body, the code escaped in the path; a 404 is no invite', async () => {
    const m = stub({ podName: 'Launch room', inviterName: 'Ada', expiresAt: '2026-11-15T10:00:00Z' });
    expect(await browserHttp.readInvite('a b/c')).toEqual({ podName: 'Launch room', inviterName: 'Ada', expiresAt: '2026-11-15T10:00:00Z' });
    expect(sent(m)).toMatchObject({ url: '/v1/invites/a%20b%2Fc', method: undefined, body: undefined });
    vi.stubGlobal('fetch', vi.fn(async () => new Response('{"code":"not_found","message":"No such invite."}', { status: 404 })));
    expect(await browserHttp.readInvite('x')).toBeNull();
  });
});

describe('browserHttp podAccess', () => {
  const answering = (status: number) => vi.stubGlobal('fetch', vi.fn(async () => new Response('{}', { status })));

  it('GETs the pod\'s whoami over the cookie, the id escaped in the path', async () => {
    const m = stub({ podId: 'pod 1' });
    expect(await browserHttp.podAccess('pod 1')).toBe('member');
    const [url, init] = m.mock.calls[0] as unknown as [string, RequestInit];
    expect({ url, method: init.method ?? 'GET', credentials: init.credentials }).toEqual({ url: '/v1/pods/pod%201/me', method: 'GET', credentials: 'same-origin' });
  });

  it('reads 404 and 403 as refused and 401 as signed out, and throws on anything else', async () => {
    answering(404);
    expect(await browserHttp.podAccess('p')).toBe('refused');
    answering(403);
    expect(await browserHttp.podAccess('p')).toBe('refused');
    answering(401);
    expect(await browserHttp.podAccess('p')).toBe('signed-out');
    answering(502);
    await expect(browserHttp.podAccess('p')).rejects.toMatchObject({ status: 502 });
  });
});

describe('browserHttp stopRun', () => {
  it('POSTs the run\'s stop route over the cookie as JSON, ids escaped in the path', async () => {
    const m = stub({ runId: 'run_1', outcome: 'cancelled' });
    await browserHttp.stopRun('pod 1', 'run/1');
    expect(sent(m)).toEqual({ url: '/v1/pods/pod%201/runs/run%2F1/stop', method: 'POST', type: 'application/json', body: '{}', credentials: 'same-origin' });
  });

  it('throws the server\'s message on a refusal, and a plain one when the body is not an error', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({ code: 'forbidden', message: 'Only the agent\'s owner or a pod owner may stop a run.' }), { status: 403 })));
    await expect(browserHttp.stopRun('p', 'r')).rejects.toMatchObject({ status: 403, message: 'Only the agent\'s owner or a pod owner may stop a run.' });
    vi.stubGlobal('fetch', vi.fn(async () => new Response('<html>', { status: 502 })));
    await expect(browserHttp.stopRun('p', 'r')).rejects.toMatchObject({ status: 502, message: 'request failed' });
  });
});

describe('browserHttp logout', () => {
  it('POSTs /v1/auth/logout over the cookie as JSON, which the server asks of every cookie write', async () => {
    const m = stub({});
    await browserHttp.logout();
    expect(sent(m)).toEqual({ url: '/v1/auth/logout', method: 'POST', type: 'application/json', body: '{}', credentials: 'same-origin' });
  });

  it('throws on a refusal, so the page stays signed in', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({ code: 'forbidden', message: 'A browser session sends JSON.' }), { status: 403 })));
    await expect(browserHttp.logout()).rejects.toMatchObject({ status: 403 });
  });
});
