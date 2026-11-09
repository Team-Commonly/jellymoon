// Where an upstream may be, and which adapter speaks to it. One adapter file per protocol, each with list and call; a new
// protocol (OpenAPI, plain REST) is a new file and one line in ADAPTERS (ADR 0009).
import type { Pin } from './catalogue';
import * as mcpHttp from './upstream-mcp';

export type Adapter = typeof mcpHttp;
const ADAPTERS: Record<Pin['protocol'], Adapter> = { 'mcp-http': mcpHttp };
export const adapterOf = (pin: Pin): Adapter => ADAPTERS[pin.protocol];

/** An IPv4 literal in a loopback, private, link-local, carrier-grade NAT or unspecified range. */
function privateV4(host: string): boolean {
  const m = /^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/.exec(host);
  if (m === null) return false;
  const [a, b] = [Number(m[1]), Number(m[2])];
  return a === 0 || a === 10 || a === 127 || (a === 169 && b === 254) || (a === 172 && b >= 16 && b <= 31) || (a === 192 && b === 168) || (a === 100 && b >= 64 && b <= 127);
}

/** An IPv6 literal (URL.hostname keeps the brackets) that is loopback, unspecified, unique-local, link-local or v4-mapped. */
const privateV6 = (host: string): boolean => host.startsWith('[') && /^\[(::1?|f[cd][0-9a-f]*:.*|fe[89ab][0-9a-f]*:.*|::ffff:.*)\]$/i.test(host);

/**
 * Why `raw` may not be an upstream, or null. https only, no credentials in the URL, and no loopback, private or link-local host
 * outside ENV=local (where the e2e stub runs on 127.0.0.1, over http). A name that resolves to a private address is not caught
 * here; see the ADR's open questions.
 */
export function urlProblem(raw: string, local: boolean): string | null {
  let u: URL;
  try { u = new URL(raw); } catch { return 'url is not a URL.'; }
  if (u.username !== '' || u.password !== '') return 'url must not carry credentials; send the token in token.';
  if (local) return u.protocol === 'https:' || u.protocol === 'http:' ? null : 'url must be http or https.';
  if (u.protocol !== 'https:') return 'url must be https.';
  const host = u.hostname.toLowerCase();
  if (host === 'localhost' || host.endsWith('.localhost') || privateV4(host) || privateV6(host)) return 'url must not name a loopback, private or link-local host.';
  return null;
}
