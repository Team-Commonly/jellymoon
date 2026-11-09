// Every SQL statement the server runs. Tables: pods (core state as text), events (the sinceSeq log), tokens (hashes only),
// users, computers, invites (code hashes only), sign-in: device codes and OAuth states (hashes only), GitHub ids, and the
// connector layer's connections (credentials as envelopes only), grants and connector_calls (the receipts; a parked write is a
// row with outcome parked, ADR 0009). A command's state and events are written in one transaction by `savePod`.
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
create table if not exists invites (hash text primary key, pod text not null, uses_left integer not null, expires_at double precision not null);
create table if not exists device_codes (hash text primary key, user_hash text not null unique, expires_at double precision not null,
  last_poll double precision not null default 0, user_id text, exchanged boolean not null default false);
create table if not exists oauth_states (hash text primary key, next text not null, expires_at double precision not null);
create table if not exists github_users (github_id text primary key, user_id text not null);
create table if not exists connections (id text primary key, pod text not null, vendor text not null, url text not null, kid text not null, wrapped text not null, sealed text not null, created_at double precision not null);
create table if not exists grants (connection text not null, agent text not null, access text not null, created_at double precision not null, primary key (connection, agent));
create table if not exists connector_calls (id text primary key, pod text not null, connection text not null, operation text not null, args text not null, args_hash text not null,
  run_id text not null, agent_id text not null, outcome text not null, upstream_status integer, decision_id text unique, approver text, result text,
  created_at double precision not null, settled_at double precision, expires_at double precision);`;

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

export const saveUser = (db: Tx, u: UserRow): Promise<unknown> => db.rows('insert into users (id, name) values ($1, $2)', [u.id, u.name]);
export const allUsers = (db: Db): Promise<UserRow[]> => db.rows<UserRow>('select id, name from users');

export const saveComputer = (db: Tx, c: ComputerRow): Promise<unknown> =>
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

export interface DeviceRow { expiresAt: number; userId: string | null; exchanged: boolean }
const DEVICE = 'expires_at as "expiresAt", user_id as "userId", exchanged';
export const newDevice = (db: Db, hash: string, userHash: string, expiresAt: number): Promise<unknown> =>
  db.rows('insert into device_codes (hash, user_hash, expires_at) values ($1, $2, $3)', [hash, userHash, expiresAt]);
export const deviceByHash = async (db: Db, hash: string): Promise<DeviceRow | undefined> => (await db.rows<DeviceRow>(`select ${DEVICE} from device_codes where hash = $1`, [hash]))[0];
export const deviceByUser = async (db: Db, userHash: string): Promise<DeviceRow | undefined> =>
  (await db.rows<DeviceRow>(`select ${DEVICE} from device_codes where user_hash = $1`, [userHash]))[0];
/** Records a poll unless the last one was after `notAfter`: undefined means too soon. */
export const pollDevice = async (db: Db, hash: string, now: number, notAfter: number): Promise<DeviceRow | undefined> =>
  (await db.rows<DeviceRow>(`update device_codes set last_poll = $2 where hash = $1 and last_poll <= $3 returning ${DEVICE}`, [hash, now, notAfter]))[0];
/** The one exchange of an approved code: its user id for exactly one caller, undefined for every other. */
export const exchangeDevice = async (db: Db, hash: string): Promise<string | undefined> =>
  (await db.rows<{ userId: string }>('update device_codes set exchanged = true where hash = $1 and not exchanged and user_id is not null returning user_id as "userId"', [hash]))[0]?.userId;
export const approveDevice = async (db: Db, userHash: string, userId: string, now: number): Promise<boolean> =>
  (await db.rows('update device_codes set user_id = $2 where user_hash = $1 and user_id is null and expires_at > $3 returning hash', [userHash, userId, now])).length === 1;

export const newState = (db: Db, hash: string, next: string, expiresAt: number): Promise<unknown> =>
  db.rows('insert into oauth_states (hash, next, expires_at) values ($1, $2, $3)', [hash, next, expiresAt]);
/** A state is good once: it is deleted as it is read, and its next path comes back. */
export const takeState = async (db: Db, hash: string, now: number): Promise<string | undefined> =>
  (await db.rows<{ next: string }>('delete from oauth_states where hash = $1 and expires_at > $2 returning next', [hash, now]))[0]?.next;

export const githubUserId = async (db: Db, githubId: string): Promise<string | undefined> =>
  (await db.rows<{ userId: string }>('select user_id as "userId" from github_users where github_id = $1', [githubId]))[0]?.userId;
export const saveGithubUser = (t: Tx, githubId: string, userId: string): Promise<unknown> =>
  t.rows('insert into github_users (github_id, user_id) values ($1, $2)', [githubId, userId]);

// ---- the connector layer (ADR 0009) ----
export interface ConnRow { id: string; pod: string; vendor: string; url: string; kid: string; wrapped: string; sealed: string; createdAt: number }
export interface GrantRow { connection: string; agent: string; access: string; createdAt: number }
export interface CallRow {
  id: string; pod: string; connection: string; operation: string; args: string; argsHash: string; runId: string; agentId: string; outcome: string;
  upstreamStatus: number | null; decisionId: string | null; approver: string | null; result: string | null; createdAt: number; settledAt: number | null; expiresAt: number | null;
}
const CONN = 'id, pod, vendor, url, kid, wrapped, sealed, created_at as "createdAt"';
const CALL = `id, pod, connection, operation, args, args_hash as "argsHash", run_id as "runId", agent_id as "agentId", outcome, upstream_status as "upstreamStatus",
  decision_id as "decisionId", approver, result, created_at as "createdAt", settled_at as "settledAt", expires_at as "expiresAt"`;

export const saveConnection = (db: Db, r: ConnRow): Promise<unknown> =>
  db.rows('insert into connections (id, pod, vendor, url, kid, wrapped, sealed, created_at) values ($1, $2, $3, $4, $5, $6, $7, $8)', [r.id, r.pod, r.vendor, r.url, r.kid, r.wrapped, r.sealed, r.createdAt]);
export async function findConnection(db: Db, pod: string, id: string): Promise<ConnRow | undefined> {
  const [r] = await db.rows<ConnRow>(`select ${CONN} from connections where pod = $1 and id = $2`, [pod, id]);
  return r;
}
export const connectionsOf = (db: Db, pod: string): Promise<ConnRow[]> => db.rows<ConnRow>(`select ${CONN} from connections where pod = $1 order by created_at, id`, [pod]);

/** A second grant to the same agent on the same connection replaces the first. */
export const saveGrant = (db: Db, g: GrantRow): Promise<unknown> =>
  db.rows('insert into grants (connection, agent, access, created_at) values ($1, $2, $3, $4) on conflict (connection, agent) do update set access = excluded.access, created_at = excluded.created_at', [g.connection, g.agent, g.access, g.createdAt]);
export async function findGrant(db: Db, connection: string, agent: string): Promise<GrantRow | undefined> {
  const [r] = await db.rows<GrantRow>('select connection, agent, access, created_at as "createdAt" from grants where connection = $1 and agent = $2', [connection, agent]);
  return r;
}

export const saveCall = (t: Tx, r: CallRow): Promise<unknown> =>
  t.rows(`insert into connector_calls (id, pod, connection, operation, args, args_hash, run_id, agent_id, outcome, upstream_status, decision_id, approver, result, created_at, settled_at, expires_at)
    values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)`,
  [r.id, r.pod, r.connection, r.operation, r.args, r.argsHash, r.runId, r.agentId, r.outcome, r.upstreamStatus, r.decisionId, r.approver, r.result, r.createdAt, r.settledAt, r.expiresAt]);

/**
 * The one place a parked write is consumed: it moves from parked to `outcome` in the caller's transaction, and only a row
 * still parked and unexpired at `now` moves. The row comes back exactly once; every later claim of it finds nothing.
 */
export async function claimParked(t: Tx, decisionId: string, outcome: 'released' | 'rejected', approver: string | null, now: number): Promise<CallRow | undefined> {
  const [r] = await t.rows<CallRow>(`update connector_calls set outcome = $2, approver = $3, settled_at = $4 where decision_id = $1 and outcome = 'parked' and expires_at > $4 returning ${CALL}`, [decisionId, outcome, approver, now]);
  return r;
}

/** What the upstream answered to a released write. */
export const finishCall = (db: Db, id: string, outcome: string, status: number | null, result: string | null): Promise<unknown> =>
  db.rows('update connector_calls set outcome = $2, upstream_status = $3, result = $4 where id = $1', [id, outcome, status, result]);

export const callsOf = (db: Db, pod: string, decisionId: string | undefined, limit: number): Promise<CallRow[]> => (decisionId === undefined
  ? db.rows<CallRow>(`select ${CALL} from connector_calls where pod = $1 order by created_at desc, id desc limit $2`, [pod, limit])
  : db.rows<CallRow>(`select ${CALL} from connector_calls where pod = $1 and decision_id = $2 limit $3`, [pod, decisionId, limit]));
