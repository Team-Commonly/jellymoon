// Ids are a type prefix plus a ULID (monotonic within one object, so a task list sorts in creation order).
// Tokens are a kind prefix plus random base62; only their SHA-256 is stored.
const B32 = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';
const B62 = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz';
let lastTime = 0;
let lastRand = 0n;

const bytes = (n: number): Uint8Array => crypto.getRandomValues(new Uint8Array(n));
const hex = (b: Uint8Array): string => Array.from(b, (x) => x.toString(16).padStart(2, '0')).join('');

function encode(t: number, rand: bigint): string {
  let time = t;
  let head = '';
  for (let i = 0; i < 10; i += 1, time = Math.floor(time / 32)) head = B32[time % 32] + head;
  let rest = rand;
  let tail = '';
  for (let i = 0; i < 16; i += 1, rest /= 32n) tail = B32[Number(rest % 32n)] + tail;
  return head + tail;
}

function ulid(): string {
  const t = Math.max(Date.now(), lastTime);
  lastRand = t === lastTime ? lastRand + 1n : BigInt(`0x${hex(bytes(10))}`);
  lastTime = t;
  return encode(t, lastRand);
}

export const newId = (prefix: string): string => prefix + ulid();

/**
 * The seed the core derives a command's wake ids from (podcore nthWakeId adds 0, 1, 2... to its random part). Not monotonic:
 * the next command's seed must never land inside a range a pending wake already took. The random part is 72 bits, so the
 * additions cannot overflow it.
 */
export const newWakeSeed = (): string => `wak_${encode(Date.now(), BigInt(`0x${hex(bytes(9))}`))}`;

/** `n` random base62 characters, uniform (bytes of 248 or more are discarded). */
export function base62(n: number): string {
  let out = '';
  while (out.length < n) for (const b of bytes(n * 2)) if (b < 248 && out.length < n) out += B62[b % 62];
  return out;
}

export const newToken = (prefix: string): string => prefix + base62(43);
export const newInviteCode = (): string => `inv_${base62(22)}`;

export async function hashToken(token: string): Promise<string> {
  return hex(new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(token))));
}
