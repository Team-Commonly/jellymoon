// The HTTP client the screens use. Injected like the socket, so tests pass a fake and never touch a network.
import type { AcceptInviteResponse, Actor } from '../../gen/ts/protocol';

export interface Http {
  acceptInvite(code: string): Promise<AcceptInviteResponse>;
  /** The person the session cookie belongs to, or null when nobody is signed in. */
  me(): Promise<Actor | null>;
  approveDevice(userCode: string): Promise<void>;
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
  async me() {
    const res = await fetch('/v1/me', { credentials: 'same-origin' });
    if (res.status === 401) return null;
    if (!res.ok) throw new ApiError(res.status, 'request failed');
    return (await res.json()) as Actor;
  },
  async approveDevice(userCode) {
    const res = await fetch('/v1/auth/device/approve', { method: 'POST', credentials: 'same-origin', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ userCode }) });
    if (!res.ok) throw new ApiError(res.status, 'request failed');
  },
};

/** A non-2xx answer. The status is what screens branch on. */
export class ApiError extends Error {
  constructor(readonly status: number, message: string) {
    super(message);
  }
}
