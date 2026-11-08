// Every SQL statement the server runs. Tables: pods (core state as text), events (the sinceSeq log), tokens (hashes only),
// users, computers, invites (code hashes only). A command's state and events are written in one transaction by `savePod`.
// State is text, not jsonb: jsonb reorders object keys, and creation order is the order of the state's keys.
import type { AnyEvent } from '../../gen/ts/protocol';
import type { PodState } from '../../podcore/src/index';
import { json } from './db';
import type { Db, Tx } from './db';

export interface PodMeta { name: string; createdAt: string; harness: Record<string, string> }
export interface PodRow { id: string; state: PodState; meta: PodMeta }
export interface UserRow { id: string; name: string }
export interface ComputerRow { id: string; userId: string; name: string; os: string; arch: string }
export interface TokenRow { kind: string; subject: string; pod: string | null }
export interface InviteRow { hash: string; pod: string; usesLeft: number; expiresAt: number }

const SCHEMA = `
create table if not exists pods (id text primary key, state text not null, meta text not null);
create table if not exists events (pod text not null, seq integer not null, envelope text not null, primary key (pod, seq));
create table if not exists tokens (hash text primary key, kind text not null, subject text not null, pod text, revoked_at double precision);
create table if not exists users (id text primary key, name text not null);
create table if not exists computers (id text primary key, user_id text not null, name text not null, os text not null, arch text not null);
create table if not exists invites (hash text primary key, pod text not null, uses_left integer not null, expires_at double precision not null);`;

export const migrate = (db: Db): Promise<void> => db.exec(SCHEMA);

export async function loadPod(db: Db, id: string): Promise<PodRow | undefined> {
  const [r] = await db.rows<{ id: string; state: unknown; meta: unknown }>('select id, state, meta from pods where id = $1', [id]);
  return r === undefined ? undefined : { id: r.id, state: json<PodState>(r.state), meta: json<PodMeta>(r.meta) };
}

export async function loadPods(db: Db): Promise<PodRow[]> {
  const rows = await db.rows<{ id: string; state: unknown; meta: unknown }>('select id, state, meta from pods');
  return rows.map((r) => ({ id: r.id, state: json<PodState>(r.state), meta: json<PodMeta>(r.meta) }));
}

/** The new state and every members-event it produced, in one transaction. */
export async function savePod(t: Tx, row: PodRow, events: AnyEvent[]): Promise<void> {
  await t.rows('insert into pods (id, state, meta) values ($1, $2, $3) on conflict (id) do update set state = excluded.state, meta = excluded.meta',
    [row.id, JSON.stringify(row.state), JSON.stringify(row.meta)]);
  if (events.length === 0) return;
  const marks = events.map((_, i) => `($1, $${i * 2 + 2}, $${i * 2 + 3})`).join(', ');
  await t.rows(`insert into events (pod, seq, envelope) values ${marks}`, [row.id, ...events.flatMap((e) => [e.seq, JSON.stringify(e)])]);
}

export async function eventsAfter(db: Db, pod: string, since: number, limit: number): Promise<AnyEvent[]> {
  const rows = await db.rows<{ envelope: unknown }>('select envelope from events where pod = $1 and seq > $2 order by seq limit $3', [pod, since, limit]);
  return rows.map((r) => json<AnyEvent>(r.envelope));
}

export const saveToken = (t: Tx, hash: string, kind: string, subject: string, pod?: string): Promise<unknown> =>
  t.rows('insert into tokens (hash, kind, subject, pod) values ($1, $2, $3, $4)', [hash, kind, subject, pod ?? null]);

export async function findToken(db: Db, hash: string): Promise<TokenRow | undefined> {
  const [r] = await db.rows<TokenRow>('select kind, subject, pod from tokens where hash = $1 and revoked_at is null', [hash]);
  return r;
}

export const saveUser = (db: Db, u: UserRow): Promise<unknown> => db.rows('insert into users (id, name) values ($1, $2)', [u.id, u.name]);
export const allUsers = (db: Db): Promise<UserRow[]> => db.rows<UserRow>('select id, name from users');

export const saveComputer = (db: Db, c: ComputerRow): Promise<unknown> =>
  db.rows('insert into computers (id, user_id, name, os, arch) values ($1, $2, $3, $4, $5)', [c.id, c.userId, c.name, c.os, c.arch]);
export const allComputers = (db: Db): Promise<ComputerRow[]> =>
  db.rows<ComputerRow>('select id, user_id as "userId", name, os, arch from computers');

export const saveInvite = (db: Db, i: InviteRow): Promise<unknown> =>
  db.rows('insert into invites (hash, pod, uses_left, expires_at) values ($1, $2, $3, $4)', [i.hash, i.pod, i.usesLeft, i.expiresAt]);
export async function findInvite(db: Db, hash: string): Promise<InviteRow | undefined> {
  const [r] = await db.rows<InviteRow>('select hash, pod, uses_left as "usesLeft", expires_at as "expiresAt" from invites where hash = $1', [hash]);
  return r;
}
export const useInvite = (t: Tx, hash: string): Promise<unknown> => t.rows('update invites set uses_left = uses_left - 1 where hash = $1', [hash]);
