// Wake ids derived from one adapter-minted seed (ADR 0003). Pure string arithmetic on the ULID's 80-bit random part.
const ALPHABET = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';
const SHAPE = /^wak_[0-9A-HJKMNP-TV-Z]{26}$/;

/** The k-th wake id of a seed: k = 0 is the seed, k > 0 adds k to the random part with carry. null if the seed is malformed or the part overflows. */
export function nthWakeId(seed: string, k: number): string | null {
  if (!SHAPE.test(seed) || !Number.isInteger(k) || k < 0) return null;
  const digits = Array.from(seed.slice(4));
  let carry = k;
  for (let i = 25; i >= 10 && carry > 0; i -= 1) {
    const v = ALPHABET.indexOf(digits[i]) + carry;
    digits[i] = ALPHABET[v % 32];
    carry = Math.floor(v / 32);
  }
  return carry > 0 ? null : `wak_${digits.join('')}`;
}
