// The HTTP client the screens use. Injected like the socket, so tests pass a fake and never touch a network.
import type { AcceptInviteResponse } from '../../gen/ts/protocol';

export interface Http {
  acceptInvite(code: string): Promise<AcceptInviteResponse>;
}

/** The browser client. Cookie sign-in; the code goes in the path of one request and nowhere else. */
export const browserHttp: Http = {
  async acceptInvite(code) {
    const res = await fetch(`/v1/invites/${encodeURIComponent(code)}/accept`, { method: 'POST', credentials: 'same-origin' });
    if (!res.ok) throw new ApiError(res.status, 'request failed');
    return (await res.json()) as AcceptInviteResponse;
  },
};

/** A non-2xx answer. The status is what screens branch on. */
export class ApiError extends Error {
  constructor(readonly status: number, message: string) {
    super(message);
  }
}
