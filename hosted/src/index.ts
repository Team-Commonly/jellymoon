// The hosted instance's front Worker (ADR 0010). It decides nothing: every request, WebSocket upgrades included, goes to one
// container running the server/ bundle, and a cron keeps that container up. Container fields per @cloudflare/containers 0.3.7
// (README and dist/lib/container.d.ts): defaultPort, sleepAfter and envVars are class fields, read when the container starts.
import { Container } from "@cloudflare/containers";

const PORT = 8080; // hosted/Dockerfile EXPOSEs the same port.
const NAME = "jellymoon"; // One fixed name: one instance owns every pod and its timer (ADR 0006).

interface Env {
  JELLYMOON: DurableObjectNamespace<JellymoonServer>;
  // Cloudflare secrets, set by the deploy job (.github/workflows/hosted.yml) from GitHub Actions secrets.
  DATABASE_URL?: string;
  GITHUB_CLIENT_ID?: string;
  GITHUB_CLIENT_SECRET?: string;
  JM_VAULT_KEY?: string;
  PUBLIC_URL?: string;
  // The database client certificate (PEM) and, optionally, the name its server certificate is checked against.
  JM_DB_CLIENT_CERT?: string;
  JM_DB_CLIENT_KEY?: string;
  JM_DB_SERVER_CA?: string;
  JM_DB_TLS_SERVERNAME?: string;
  // The deployed commit, a plain var set by `wrangler deploy --var JM_BUILD:<sha>` (hosted/ship.sh).
  JM_BUILD?: string;
}

const SECRETS = [
  "DATABASE_URL", "GITHUB_CLIENT_ID", "GITHUB_CLIENT_SECRET", "JM_VAULT_KEY", "PUBLIC_URL", "JM_BUILD",
  "JM_DB_CLIENT_CERT", "JM_DB_CLIENT_KEY", "JM_DB_SERVER_CA", "JM_DB_TLS_SERVERNAME",
] as const;

export class JellymoonServer extends Container<Env> {
  defaultPort = PORT;
  // A sleeping container stops the pod timer (ADR 0010), so it never sleeps: the timeout is long, and expiry renews it
  // instead of stopping (the library's default onActivityExpired calls stop()).
  sleepAfter = "24h";
  override async onActivityExpired(): Promise<void> {}
  // ENV stays unset and there is no TEST_BEARER: the hosted shape (server/src/config.ts). A missing secret is left out, so the
  // server's own startupRefusal names it.
  envVars = {
    ...Object.fromEntries(SECRETS.flatMap((k) => (this.env[k] ? [[k, this.env[k]]] : []))),
    TRUST_PROXY: "1",
    JM_WEB_DIR: "/app/web",
    PORT: String(PORT),
  };
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const headers = new Headers(request.headers);
    // The one proxy hop the server trusts (TRUST_PROXY=1) is this one: the client's address is the only X-Forwarded-For entry,
    // so the start limit keys on the real client (ADR 0008, "Hardening" item 2).
    const ip = request.headers.get("cf-connecting-ip");
    if (ip) headers.set("x-forwarded-for", ip);
    else headers.delete("x-forwarded-for");
    // The library reads this header to pick a container port; a client never chooses one.
    headers.delete("cf-container-target-port");
    return env.JELLYMOON.getByName(NAME).fetch(new Request(request, { headers }));
  },
  async scheduled(_event: ScheduledController, env: Env, ctx: ExecutionContext): Promise<void> {
    // Starts a stopped container within a minute; the response itself is not needed.
    ctx.waitUntil(env.JELLYMOON.getByName(NAME).fetch(new Request(`http://${NAME}/healthz`)).then((r) => r.body?.cancel()));
  },
} satisfies ExportedHandler<Env>;
