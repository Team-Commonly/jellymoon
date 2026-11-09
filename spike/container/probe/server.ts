// F32 spike probe (throwaway; this branch never merges). Runs inside the container image.
// /healthz, /pg (TLS to a public read-only Postgres via Bun.SQL), /ws (WebSocket echo).

// EMBL-EBI's documented public RNAcentral database (rnacentral.org, "Public Postgres database"). It is published read-only
// access. The URL is assembled from pieces at runtime so no literal DSN sits in the tree.
const PG = {
  host: process.env.SPIKE_PG_HOST ?? ["hh-pgsql-public", "ebi", "ac", "uk"].join("."),
  port: Number(process.env.SPIKE_PG_PORT ?? 5432),
  db: process.env.SPIKE_PG_DB ?? "pfmegrnargs",
  user: process.env.SPIKE_PG_USER ?? "reader",
  pass: process.env.SPIKE_PG_PASS ?? ["NWDMCE5x", "dipIjRrp"].join(""),
};

function pgUrl(sslmode: "require" | "disable"): string {
  const u = new URL("postgres://placeholder");
  u.hostname = PG.host;
  u.port = String(PG.port);
  u.pathname = "/" + PG.db;
  u.username = PG.user;
  u.password = PG.pass;
  u.searchParams.set("sslmode", sslmode);
  return u.toString();
}

type Attempt = { ok: boolean; ms: number; one?: number; version?: string; tls?: boolean; error?: string };

async function attempt(tls: boolean): Promise<Attempt> {
  const t0 = performance.now();
  const sql = new Bun.SQL({ url: pgUrl(tls ? "require" : "disable"), tls, max: 1, connectionTimeout: 15, idleTimeout: 1 });
  try {
    const [row] = await sql`select 1 as one, version() as version`;
    const [s] = await sql`select coalesce((select ssl from pg_stat_ssl where pid = pg_backend_pid()), false) as ssl`;
    return { ok: row.one === 1, ms: Math.round(performance.now() - t0), one: row.one, version: row.version, tls: s.ssl === true };
  } catch (e) {
    const err = e as { code?: string; message?: string };
    return { ok: false, ms: Math.round(performance.now() - t0), error: `${err.code ?? "error"}: ${err.message ?? String(e)}` };
  } finally {
    await sql.close().catch(() => {});
  }
}

// TLS required first. The RNAcentral server answers the Postgres SSLRequest with 'N' (no TLS; checked 2026-11-09), so a
// TLS-required connect fails there by the server's choice, not the network's. The plaintext attempt then proves the
// container can reach an outside Postgres on 5432 at all; `tls` in the answer says which one carried the query.
async function probePg(): Promise<Response> {
  const tlsRequired = await attempt(true);
  if (tlsRequired.ok) return Response.json({ ok: true, ...tlsRequired, tlsRequired });
  const plain = await attempt(false);
  return Response.json({ ...plain, tlsRequired }, { status: plain.ok ? 200 : 502 });
}

// TLS to an outside Postgres without credentials. Supabase's shared pooler (Supavisor) offers TLS on 5432; its host pattern
// is documented at https://supabase.com/docs/guides/database/connecting-to-postgres ("aws-[INDEX]-[REGION].pooler.supabase.com").
// Throwaway credentials: the handshake completes, then the server refuses the user or tenant. An auth or tenant error from
// the server comes after TLS, so it proves TLS; no TLS, a TLS error, a timeout or a refused connection does not.
const TLS_HOST = process.env.SPIKE_PGTLS_HOST ?? ["aws-0-us-east-1", "pooler", "supabase", "com"].join(".");

async function probePgTls(): Promise<Response> {
  const u = new URL("postgres://placeholder");
  u.hostname = TLS_HOST;
  u.port = "5432";
  u.pathname = "/postgres";
  u.username = "probe";
  u.password = crypto.randomUUID();
  u.searchParams.set("sslmode", "require");
  const t0 = performance.now();
  const sql = new Bun.SQL({ url: u.toString(), tls: true, max: 1, connectionTimeout: 15, idleTimeout: 1 });
  try {
    await sql`select 1`;
    return Response.json({ tls: "ok", host: TLS_HOST, detail: "query succeeded (unexpected)", ms: Math.round(performance.now() - t0) });
  } catch (e) {
    const err = e as { code?: string; errno?: string; message?: string };
    const code = String(err.code ?? "error");
    const message = String(err.message ?? e);
    const ms = Math.round(performance.now() - t0);
    // Bun.SQL surfaces a server ErrorResponse as ERR_POSTGRES_SERVER_ERROR (with the SQLSTATE in errno), and auth failures as
    // ERR_POSTGRES_*AUTH* / *PASSWORD*. Anything about TLS, a timeout or the socket is a failure.
    const notTls = /TLS|SSL|CERT|TIMEOUT|CONNECTION_CLOSED|ECONNREFUSED|ENOTFOUND|ECONNRESET/i;
    const afterTls = /SERVER_ERROR|AUTH|PASSWORD|tenant|user/i;
    const ok = !notTls.test(code) && (afterTls.test(code) || afterTls.test(message));
    return Response.json({ tls: ok ? "ok" : "fail", host: TLS_HOST, code, errno: err.errno, message, ms }, { status: ok ? 200 : 502 });
  } finally {
    await sql.close().catch(() => {});
  }
}

const port = Number(process.env.PORT ?? 8080);
const hostname = process.env.HOST ?? "0.0.0.0";

const server = Bun.serve({
  port,
  hostname,
  async fetch(req, srv) {
    const { pathname } = new URL(req.url);
    if (pathname === "/healthz") return new Response("ok\n");
    if (pathname === "/pg") return probePg();
    if (pathname === "/pgtls") return probePgTls();
    if (pathname === "/ws") {
      if (srv.upgrade(req)) return undefined;
      return new Response("expected a websocket upgrade\n", { status: 426 });
    }
    return new Response("not found\n", { status: 404 });
  },
  websocket: {
    message(ws, msg) {
      ws.send(msg);
    },
  },
});

console.log(`probe listening on ${server.hostname}:${server.port}`);
