import { expect, test } from 'bun:test';
import type { Db } from '../src/db';
import { openDb } from '../src/db';
import { BootRefused, Hub } from '../src/hub';

// The boot lock (ADR 0010, "A rollout's overlap"): a server that cannot take its database's lock restores no pod and runs no
// statement. The two-process proof on a real Postgres is TestBootLock in the contract suite; here PGlite stands in, with the
// lock answered by the test, because PGlite is one process's own database and has no second session to hold it.
function recording(db: Db, lock: Db['bootLock']): { db: Db; seen: string[] } {
  const seen: string[] = [];
  return {
    seen,
    db: {
      rows: (sql, params) => { seen.push(sql); return db.rows(sql, params); },
      exec: (sql) => { seen.push(sql); return db.exec(sql); },
      tx: (fn) => { seen.push('tx'); return db.tx(fn); },
      bootLock: lock === undefined ? undefined : async (lost) => { seen.push('lock'); return lock(lost); },
    },
  };
}

test('a server whose lock is held elsewhere refuses, and restores no pod and runs no statement', async () => {
  const base = await openDb({});
  const first = await Hub.restore(base);
  const { user: owner } = await first.createUser('olivia');
  await first.createPod(owner, 'kept');
  const { db, seen } = recording(base, async () => 'held by another process');
  const refused = await Hub.boot(db, () => {}).then(() => undefined, (e: unknown) => e);
  expect(refused instanceof BootRefused).toBe(true);
  expect((refused as Error).message).toBe('held by another process');
  expect(seen).toEqual(['lock']);
});

test('the lock is taken before the first statement, and a held lock boots as before', async () => {
  const base = await openDb({});
  const first = await Hub.restore(base);
  await first.createPod((await first.createUser('olivia')).user, 'kept');
  const { db, seen } = recording(base, async () => undefined);
  const hub = await Hub.boot(db, () => {});
  expect(seen[0]).toBe('lock');
  expect(seen.length > 1).toBe(true);
  expect(hub.pods.size).toBe(1);
});

test('the in-process Postgres has no boot lock: the local trial boots as it did', async () => {
  const db = await openDb({});
  expect(db.bootLock).toBe(undefined);
  expect((await Hub.boot(db, () => { throw new Error('no lock to lose'); })).pods.size).toBe(0);
});
