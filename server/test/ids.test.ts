import { describe, expect, test } from 'bun:test';
import { base62, hashToken, newId, newInviteCode, newToken, newWakeSeed } from '../src/ids';

describe('ids', () => {
  test('an id is its prefix plus a 26-character ULID with fresh random bits, never a counter', () => {
    const ids = Array.from({ length: 200 }, () => newId('tsk_'));
    for (const id of ids) expect(/^tsk_[0-9A-HJKMNP-TV-Z]{26}$/.test(id)).toBe(true);
    expect(new Set(ids).size).toBe(ids.length);
    // Two ids made in the same millisecond differ in more than the last character: no +1 neighbour.
    const sameMs = ids.filter((id, i) => i > 0 && id.slice(0, 14) === ids[i - 1].slice(0, 14));
    for (const id of sameMs) expect(id.slice(14, 25) === ids[ids.indexOf(id) - 1].slice(14, 25)).toBe(false);
  });

  test('a wake seed is a wake id whose random part leaves room for the core to add to it', () => {
    const seed = newWakeSeed();
    expect(/^wak_[0-9A-HJKMNP-TV-Z]{26}$/.test(seed)).toBe(true);
    expect(newWakeSeed() === seed).toBe(false);
  });

  test('tokens are the prefix plus 43 base62 characters, and only the hash is derived from them', () => {
    const t = newToken('jmr_');
    expect(/^jmr_[0-9A-Za-z]{43}$/.test(t)).toBe(true);
    expect(newToken('jmr_') === t).toBe(false);
    expect(/^[0-9a-f]{64}$/.test(hashToken(t))).toBe(true);
    expect(hashToken(t) === t).toBe(false);
  });

  test('base62 and invite codes have the schema shapes', () => {
    expect(base62(100).length).toBe(100);
    expect(/^inv_[0-9A-Za-z]{22}$/.test(newInviteCode())).toBe(true);
  });
});
