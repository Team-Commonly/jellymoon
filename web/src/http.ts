// The HTTP client the screens use. Injected like the socket, so tests pass a fake and never touch a network.
import type { AcceptInviteResponse } from '../../gen/ts/protocol';

export interface Http {
  acceptInvite(code: string): Promise<AcceptInviteResponse>;
}

/** A non-2xx answer. The status is what screens branch on. */
export class ApiError extends Error {
  constructor(readonly status: number, message: string) {
    super(message);
  }
}
