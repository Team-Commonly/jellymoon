// F32 spike front Worker (throwaway). Forwards every request, WebSocket upgrades included, to one container instance.
import { Container } from "@cloudflare/containers";

export class Probe extends Container {
  defaultPort = 8080;
  sleepAfter = "5m";
}

interface Env {
  PROBE: DurableObjectNamespace<Probe>;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    // One fixed name: every request, and every upgrade, lands on the same instance.
    return env.PROBE.getByName("probe").fetch(request);
  },
} satisfies ExportedHandler<Env>;
