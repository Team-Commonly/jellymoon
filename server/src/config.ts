// What the process must refuse at startup, kept apart from main.ts so a test can pin it.
export function startupRefusal(env: Record<string, string | undefined>): string | undefined {
  const local = env.ENV === 'local';
  if (env.GITHUB_URL && !local && !env.GITHUB_URL.startsWith('https://')) return 'GITHUB_URL must be https; an http override is for ENV=local only.';
  if (env.DATABASE_URL === undefined || env.DATABASE_URL === '') {
    return local ? undefined : 'DATABASE_URL is required. The in-process Postgres is for local runs only (ENV=local).';
  }
  // ENV=local opens /__test/session and /__test/clock with no bearer. Against a real database that is an open door, so it is
  // refused; only a throwaway schema (the e2e suite sets JM_PG_SCHEMA per server) may combine the two.
  return local && (env.JM_PG_SCHEMA === undefined || env.JM_PG_SCHEMA === '')
    ? 'ENV=local is refused with DATABASE_URL: it opens the test routes with no bearer. Unset ENV, or confine the server to a throwaway schema with JM_PG_SCHEMA.'
    : undefined;
}

/** What buildApp is given, from the environment. Test routes open only for ENV=local or a configured TEST_BEARER; kept apart from main.ts so a test can start the app as production does. */
export function appOptions(env: Record<string, string | undefined>): { publicUrl: string | undefined; local: boolean; testBearer: string | undefined } {
  return { publicUrl: env.PUBLIC_URL, local: env.ENV === 'local', testBearer: env.TEST_BEARER };
}
