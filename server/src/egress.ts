// Where a connector may send the pod's credential (ADR 0009, "Egress"). Before every upstream request (connect's listing,
// each read, each released write) the stored url is checked again: https, no credentials, a host that is not localhost, and
// every address its name resolves to outside the internal ranges. The request is then pinned to the address that was checked:
// it is sent to that IP, with the name kept as the Host header and as the TLS server name, so the certificate is still
// verified against the name and a second lookup cannot move the request elsewhere.
import { lookup } from 'node:dns/promises';

/** A name's addresses. Injected in tests; the server uses the system resolver. */
export type Resolve = (host: string) => Promise<string[]>;
export const systemResolve: Resolve = async (host) => (await lookup(host, { all: true, verbatim: true })).map((a) => a.address);

/** What an adapter fetches: the url, and for a pinned name, the Host header and the TLS server name to send with it. */
export interface Dest { url: string; host?: string; serverName?: string }

/** A dotted-decimal IPv4 address as four octets, or null. Leading zeros are refused (they would be ambiguous). */
function v4(s: string): number[] | null {
  const p = s.split('.');
  return p.length === 4 && p.every((x) => /^(0|[1-9]\d{0,2})$/.test(x) && Number(x) <= 255) ? p.map(Number) : null;
}

/** An IPv6 address as eight 16-bit groups, or null. A trailing dotted IPv4 counts as the last two groups; a zone id is refused. */
function v6(s: string): number[] | null {
  let text = s;
  if (text.includes('.')) {
    const at = text.lastIndexOf(':');
    const tail = at < 0 ? null : v4(text.slice(at + 1));
    if (tail === null) return null;
    text = `${text.slice(0, at + 1)}${((tail[0] << 8) | tail[1]).toString(16)}:${((tail[2] << 8) | tail[3]).toString(16)}`;
  }
  const halves = text.split('::');
  if (halves.length > 2) return null;
  const part = (h: string): string[] => (h === '' ? [] : h.split(':'));
  const [head, rest] = [part(halves[0]), halves.length === 2 ? part(halves[1]) : []];
  if (halves.length === 2 && head.length + rest.length > 7) return null;
  const groups = [...head, ...(halves.length === 2 ? Array<string>(8 - head.length - rest.length).fill('0') : []), ...rest];
  return groups.length === 8 && groups.every((g) => /^[0-9a-f]{1,4}$/i.test(g)) ? groups.map((g) => parseInt(g, 16)) : null;
}

/** 0/8, 10/8, 100.64/10, 127/8, 169.254/16, 172.16/12, 192.0.0/24, 192.168/16, 198.18/15, and 224/4 and up. */
function internalV4([a, b, c]: number[]): boolean {
  return a === 0 || a === 10 || a === 127 || a >= 224 || (a === 100 && b >= 64 && b <= 127) || (a === 169 && b === 254)
    || (a === 172 && b >= 16 && b <= 31) || (a === 192 && b === 0 && c === 0) || (a === 192 && b === 168) || (a === 198 && (b === 18 || b === 19));
}

/** ::/96 (::, ::1 and v4-compatible), fc00::/7, fe80::/10, ff00::/8; v4-mapped and NAT64 64:ff9b::/96 by their embedded IPv4. */
function internalV6(g: number[]): boolean {
  const embedded = [g[6] >> 8, g[6] & 255, g[7] >> 8, g[7] & 255];
  if (g.slice(0, 6).every((x) => x === 0)) return true;
  if (g.slice(0, 5).every((x) => x === 0) && g[5] === 0xffff) return internalV4(embedded);
  if (g[0] === 0x64 && g[1] === 0xff9b && g.slice(2, 6).every((x) => x === 0)) return internalV4(embedded);
  return (g[0] & 0xfe00) === 0xfc00 || (g[0] & 0xffc0) === 0xfe80 || (g[0] & 0xff00) === 0xff00;
}

/** Whether an address (IPv4, or IPv6 with or without brackets) is internal. Anything that does not parse counts as internal. */
export function internalAddress(addr: string): boolean {
  const a = v4(addr);
  if (a !== null) return internalV4(a);
  const g = v6(addr.replace(/^\[(.*)\]$/, '$1'));
  return g === null ? true : internalV6(g);
}

const loopbackLiteral = (h: string): boolean => v4(h)?.[0] === 127 || v6(h)?.every((x, i) => x === (i === 7 ? 1 : 0)) === true;

/**
 * The checked, pinned destination for `raw`, or why the credential may not be sent there. ENV=local alone allows a literal
 * loopback address (the e2e stub on 127.0.0.1), over http or https; every other host gets the full check in every mode.
 */
export async function destOf(raw: string, local: boolean, resolve: Resolve): Promise<Dest | string> {
  let u: URL;
  try { u = new URL(raw); } catch { return 'url is not a URL.'; }
  if (u.username !== '' || u.password !== '') return 'url must not carry credentials; send the token in token.';
  const host = u.hostname.toLowerCase().replace(/\.+$/, '').replace(/^\[(.*)\]$/, '$1');
  if (local && (u.protocol === 'http:' || u.protocol === 'https:') && loopbackLiteral(host)) return { url: u.href };
  if (u.protocol !== 'https:') return 'url must be https.';
  if (host === 'localhost' || host.endsWith('.localhost')) return 'url must not name localhost.';
  const literal = v4(host) !== null || v6(host) !== null;
  let addrs: string[];
  try { addrs = literal ? [host] : await resolve(host); } catch { addrs = []; }
  if (addrs.length === 0) return `url's host ${host} could not be resolved.`;
  if (addrs.some(internalAddress)) return `url's host ${host} is or resolves to an internal address (loopback, private, link-local or reserved).`;
  if (literal) return { url: u.href };
  const pinned = new URL(u.href);
  pinned.hostname = addrs[0].includes(':') ? `[${addrs[0]}]` : addrs[0];
  return { url: pinned.href, host: u.port === '' ? host : `${host}:${u.port}`, serverName: host };
}
