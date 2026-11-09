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

  it('me: a read with the cookie; 401 is nobody', async () => {
    const m = stub({ kind: 'human', id: 'usr_1', displayName: 'Ada' });
    expect(await browserHttp.me()).toMatchObject({ id: 'usr_1' });
    expect(sent(m)).toMatchObject({ url: '/v1/me', method: undefined, credentials: 'same-origin' });
    vi.stubGlobal('fetch', vi.fn(async () => new Response('{}', { status: 401 })));
    expect(await browserHttp.me()).toBeNull();
  });

  it('me: hands the caller\'s abort signal to fetch', async () => {
    const m = stub({ kind: 'human', id: 'usr_1', displayName: 'Ada' });
    const ctl = new AbortController();
    await browserHttp.me(ctl.signal);
    expect((m.mock.calls[0] as unknown as [string, RequestInit])[1].signal).toBe(ctl.signal);
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
