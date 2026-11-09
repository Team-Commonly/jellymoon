// Bindings and variables. TEST_BEARER and BOOT_SWEEP are read only by the test entry; the production bundle reads ENV only to
// relax https on the sign-in routes under ENV=local (ADR 0008).
export interface Env {
  DB: D1Database;
  POD: DurableObjectNamespace;
  /** Public base URL: the run's mcpUrl and an invite's join link. Optional; a request's own origin stands in. */
  PUBLIC_URL?: string;
  ENV?: string;
  TEST_BEARER?: string;
  BOOT_SWEEP?: string;
  /** Sign-in (ADR 0008): the GitHub OAuth client, and an override base for GitHub's endpoints (https, or anything under ENV=local). */
  GITHUB_CLIENT_ID?: string;
  GITHUB_CLIENT_SECRET?: string;
  GITHUB_URL?: string;
  /** The sign-in routes' clock, set by the test entry from /__test/clock; production reads Date.now(). */
  CLOCK_MS?: number;
}
