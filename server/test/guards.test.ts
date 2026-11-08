import { describe, expect, test } from 'bun:test';
import { sameSecret } from '../src/ids';
import { startupRefusal } from '../src/config';

describe('guards', () => {
  test('a secret compares equal only to itself, whatever the lengths', () => {
    expect(sameSecret('Bearer sekret', 'Bearer sekret')).toBe(true);
    expect(sameSecret('Bearer sekret', 'Bearer sekreT')).toBe(false);
    expect(sameSecret('Bearer sekret', 'Bearer')).toBe(false);
    expect(sameSecret('', 'x')).toBe(false);
  });

  test('ENV=local with a real database is refused unless the run is schema-scoped; no database needs ENV=local', () => {
    expect(startupRefusal({ ENV: 'local', DATABASE_URL: 'postgres://h/db' })?.includes('ENV=local')).toBe(true);
    expect(startupRefusal({ ENV: 'local', DATABASE_URL: 'postgres://h/db', JM_PG_SCHEMA: 'e2e_1' })).toBeUndefined();
    expect(startupRefusal({ DATABASE_URL: 'postgres://h/db' })).toBeUndefined();
    expect(startupRefusal({ ENV: 'local' })).toBeUndefined();
    expect(startupRefusal({})?.includes('DATABASE_URL')).toBe(true);
  });
});
