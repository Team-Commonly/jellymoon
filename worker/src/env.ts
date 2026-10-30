// Bindings and variables. ENV, TEST_BEARER and BOOT_SWEEP are read only by the test entry; the production bundle never looks at them.
export interface Env {
  DB: D1Database;
  POD: DurableObjectNamespace;
  /** Public base URL: the run's mcpUrl and an invite's join link. Optional; a request's own origin stands in. */
  PUBLIC_URL?: string;
  ENV?: string;
  TEST_BEARER?: string;
  BOOT_SWEEP?: string;
}
