import { describe, expect, test } from 'bun:test';
import { sameSecret } from '../src/ids';
import { dbTlsOf, startupRefusal } from '../src/config';

describe('guards', () => {
  test('a secret compares equal only to itself, whatever the lengths', () => {
    expect(sameSecret('Bearer sekret', 'Bearer sekret')).toBe(true);
    expect(sameSecret('Bearer sekret', 'Bearer sekreT')).toBe(false);
    expect(sameSecret('Bearer sekret', 'Bearer')).toBe(false);
    expect(sameSecret('', 'x')).toBe(false);
  });

  test('ENV=local with a real database is refused unless the run is schema-scoped; no database needs ENV=local', () => {
    const refused = startupRefusal({ ENV: 'local', DATABASE_URL: 'postgres://h/db' });
    expect(refused?.includes('ENV=local')).toBe(true);
    expect(refused?.includes('TEST_BEARER')).toBe(false);
    expect(startupRefusal({ ENV: 'local', DATABASE_URL: 'postgres://h/db', JM_PG_SCHEMA: 'e2e_1' }) === undefined).toBe(true);
    expect(startupRefusal({ DATABASE_URL: 'postgres://h/db' }) === undefined).toBe(true);
    expect(startupRefusal({ ENV: 'local' }) === undefined).toBe(true);
    expect(startupRefusal({})?.includes('DATABASE_URL')).toBe(true);
  });

  test('a database client certificate is all three PEMs or none, and a partial or non-PEM set refuses to start', () => {
    const pem = (t: string): string => `-----BEGIN ${t}-----\nAAAA\n-----END ${t}-----\n`;
    const full = { JM_DB_CLIENT_CERT: pem('CERTIFICATE'), JM_DB_CLIENT_KEY: pem('RSA PRIVATE KEY'), JM_DB_SERVER_CA: pem('CERTIFICATE') };
    expect(dbTlsOf({})).toBe(undefined);
    expect(dbTlsOf(full)).toEqual({ cert: full.JM_DB_CLIENT_CERT, key: full.JM_DB_CLIENT_KEY, ca: full.JM_DB_SERVER_CA });
    expect(dbTlsOf({ ...full, JM_DB_TLS_SERVERNAME: ' db.example ' })).toMatchObject({ servername: 'db.example' });
    const partial = dbTlsOf({ JM_DB_CLIENT_CERT: full.JM_DB_CLIENT_CERT });
    expect(typeof partial === 'string' && partial.includes('missing JM_DB_CLIENT_KEY, JM_DB_SERVER_CA')).toBe(true);
    const notPem = dbTlsOf({ ...full, JM_DB_CLIENT_KEY: 'c2VjcmV0' });
    expect(typeof notPem === 'string' && notPem.startsWith('JM_DB_CLIENT_KEY must be PEM')).toBe(true);
    expect(startupRefusal({ DATABASE_URL: 'postgres://h/db', JM_DB_SERVER_CA: full.JM_DB_SERVER_CA })?.includes('missing')).toBe(true);
    expect(startupRefusal({ DATABASE_URL: 'postgres://h/db', ...full })).toBe(undefined);
  });
});
