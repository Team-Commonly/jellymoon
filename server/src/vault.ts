// The connector vault (ADR 0009): a connection's credential is stored only as an AES-256-GCM envelope. Each row gets a fresh
// data key; the data key is wrapped by the master key, and the row records which master key (its id) so the ring can rotate.
// Both layers bind the row's own id as additional data, so a ciphertext copied onto another connection does not open.
// The plaintext lives only in memory, for one upstream call; nothing here logs, returns or throws it.

/** What a row stores: the master key's id, the wrapped data key and the sealed credential, each IV-prefixed base64. */
interface Sealed { kid: string; wrapped: string; sealed: string }

export interface Vault {
  seal(plain: string, aad: string): Promise<Sealed>;
  open(s: Sealed, aad: string): Promise<string>;
}

const enc = new TextEncoder();
const b64 = (b: Uint8Array): string => btoa(String.fromCharCode(...b));
const unb64 = (s: string): Uint8Array<ArrayBuffer> => Uint8Array.from(atob(s), (ch) => ch.charCodeAt(0));
const B64_32 = /^[A-Za-z0-9+/]{43}=$/;

/**
 * The key ring from the environment: JM_VAULT_KEY is the current master key, base64 of 32 bytes; JM_VAULT_OLD_KEYS is a
 * comma-separated list of retired ones that still open older rows. Returns the keys (current first), none when JM_VAULT_KEY
 * is unset, or a refusal that never echoes a key.
 */
export function keyRing(env: Record<string, string | undefined>): Uint8Array<ArrayBuffer>[] | string {
  const cur = env.JM_VAULT_KEY ?? '';
  if (cur === '') return [];
  const all = [cur, ...(env.JM_VAULT_OLD_KEYS ?? '').split(',').map((s) => s.trim()).filter((s) => s !== '')];
  return all.every((k) => B64_32.test(k)) ? all.map(unb64) : 'JM_VAULT_KEY and each of JM_VAULT_OLD_KEYS must be base64 of exactly 32 bytes.';
}

async function gcm(raw: Uint8Array<ArrayBuffer>, use: 'encrypt' | 'decrypt'): Promise<CryptoKey> {
  return crypto.subtle.importKey('raw', raw, { name: 'AES-GCM' }, false, [use]);
}

async function lock(raw: Uint8Array<ArrayBuffer>, data: Uint8Array<ArrayBuffer>, aad: string): Promise<string> {
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const ct = new Uint8Array(await crypto.subtle.encrypt({ name: 'AES-GCM', iv, additionalData: enc.encode(aad) }, await gcm(raw, 'encrypt'), data));
  return b64(new Uint8Array([...iv, ...ct]));
}

async function unlock(raw: Uint8Array<ArrayBuffer>, text: string, aad: string): Promise<Uint8Array<ArrayBuffer>> {
  const all = unb64(text);
  const out = await crypto.subtle.decrypt({ name: 'AES-GCM', iv: all.slice(0, 12), additionalData: enc.encode(aad) }, await gcm(raw, 'decrypt'), all.slice(12));
  return new Uint8Array(out);
}

/** A key's id: the first 16 hex digits of its SHA-256. It names the key without revealing it. */
async function kidOf(raw: Uint8Array<ArrayBuffer>): Promise<string> {
  const h = new Uint8Array(await crypto.subtle.digest('SHA-256', raw));
  return Array.from(h.slice(0, 8), (x) => x.toString(16).padStart(2, '0')).join('');
}

/** A vault over the ring: seals with the first key, opens with whichever key the row names. */
export async function makeVault(ring: Uint8Array<ArrayBuffer>[]): Promise<Vault> {
  const keys = new Map<string, Uint8Array<ArrayBuffer>>();
  for (const k of ring) keys.set(await kidOf(k), k);
  const [kid] = keys.keys();
  return {
    async seal(plain, aad) {
      const dk = crypto.getRandomValues(new Uint8Array(32));
      try {
        return { kid, wrapped: await lock(keys.get(kid)!, dk, aad), sealed: await lock(dk, enc.encode(plain), aad) };
      } finally { dk.fill(0); }
    },
    async open(s, aad) {
      const master = keys.get(s.kid);
      if (master === undefined) throw new Error('vault: the row was sealed under a key this server does not hold');
      const dk = await unlock(master, s.wrapped, aad);
      try {
        return new TextDecoder().decode(await unlock(dk, s.sealed, aad));
      } finally { dk.fill(0); }
    },
  };
}
