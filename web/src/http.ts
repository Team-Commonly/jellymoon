// The HTTP client the screens use. Injected like the socket, so tests pass a fake and never touch a network.
import type { AcceptInviteResponse, Actor, Pod } from '../../gen/ts/protocol';

export interface Http {
  acceptInvite(code: string): Promise<AcceptInviteResponse>;
  /** The person the session cookie belongs to, or null when nobody is signed in. */
  me(signal?: AbortSignal): Promise<Actor | null>;
  approveDevice(userCode: string): Promise<void>;
  /** Stops a run now (the agent's owner or a pod owner may). Resolves when the server accepted it; the run's end arrives as run.ended on the pod socket. A refusal throws an ApiError carrying the server's message. */
  stopRun(podId: string, runId: string): Promise<void>;
  /** Makes a pod owned by the signed-in person. A refusal throws an ApiError carrying the server's own message. */
  createPod(name: string): Promise<Pod>;
}

/** The GitHub sign-in, which brings the person back to `path` (a same-origin path) with the session cookie set. */
export const signInHref = (path: string): string => `/v1/auth/github/start?next=${encodeURIComponent(path)}`;

/** The browser client. Cookie sign-in, and every write sends JSON (the server refuses a cookie write that is not). An invite code goes in the path of one request and in a same-origin `next`, nowhere else. */
export const browserHttp: Http = {
  async acceptInvite(code) {
    const res = await fetch(`/v1/invites/${encodeURIComponent(code)}/accept`, { method: 'POST', credentials: 'same-origin', headers: { 'content-type': 'application/json' }, body: '{}' });
    if (!res.ok) throw new ApiError(res.status, 'request failed');
    return (await res.json()) as AcceptInviteResponse;
  },
  async me(signal) {
    const res = await fetch('/v1/me', { credentials: 'same-origin', signal });
    if (res.status === 401) return null;
    if (!res.ok) throw new ApiError(res.status, 'request failed');
    return (await res.json()) as Actor;
  },
  async approveDevice(userCode) {
    const res = await fetch('/v1/auth/device/approve', { method: 'POST', credentials: 'same-origin', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ userCode }) });
    if (!res.ok) throw new ApiError(res.status, 'request failed');
  },
  async stopRun(podId, runId) {
    const res = await fetch(`/v1/pods/${encodeURIComponent(podId)}/runs/${encodeURIComponent(runId)}/stop`, { method: 'POST', credentials: 'same-origin', headers: { 'content-type': 'application/json' }, body: '{}' });
    if (!res.ok) throw new ApiError(res.status, await refusalMessage(res));
  },
  async createPod(name) {
    const res = await fetch('/v1/pods', { method: 'POST', credentials: 'same-origin', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ name }) });
    if (!res.ok) {
      const body = (await res.json().catch(() => null)) as { message?: unknown } | null;
      throw new ApiError(res.status, typeof body?.message === 'string' && body.message !== '' ? body.message : `the server returned an error (${res.status}). Try again.`);
    }
    return (await res.json()) as Pod;
  },
};

/** The server's own words from a refusal ({code, message}), or a plain line when the body is not one. */
async function refusalMessage(res: Response): Promise<string> {
  try {
    const body = (await res.json()) as { message?: unknown };
    if (typeof body.message === 'string' && body.message.trim() !== '') return body.message;
  } catch { /* not JSON: fall through */ }
  return 'request failed';
}

/** How long the page waits for GET /v1/me before it renders as signed out. A server that takes the connection and never answers must not leave the page blank. */
const BOOT_ME_TIMEOUT_MS = 5000;

/**
 * The boot decision: is the session cookie a signed-in person? Nobody, an unreachable server and a server that has not answered
 * within `timeoutMs` all count as signed out. The request is aborted at the timeout, and the answer is also raced against it,
 * so a client that ignores the signal still lets the page render.
 */
export async function bootSignedIn(http: Http, timeoutMs: number = BOOT_ME_TIMEOUT_MS): Promise<boolean> {
  const ctl = new AbortController();
  const timer = setTimeout(() => ctl.abort(), timeoutMs);
  const timedOut = new Promise<null>((resolve) => ctl.signal.addEventListener('abort', () => resolve(null)));
  try {
    return (await Promise.race([http.me(ctl.signal), timedOut])) !== null;
  } catch {
    return false;
  } finally {
    clearTimeout(timer);
  }
}

/** A non-2xx answer. The status is what screens branch on. */
export class ApiError extends Error {
  constructor(readonly status: number, message: string) {
    super(message);
  }
}
