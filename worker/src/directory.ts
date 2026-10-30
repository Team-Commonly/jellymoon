// D1: the cross-pod index. Tokens and invite codes (SHA-256 only), users, computers, the pod list, and which pods a computer has agents in.
// Everything inside a pod lives in that pod's Durable Object, not here.
import type { Actor } from '../../gen/ts/protocol';
import { hashToken, newId, newToken } from './ids';
import type { Who } from './wire';

const KIND: Record<string, 'user' | 'computer' | 'run'> = { jmu_: 'user', jmc_: 'computer', jmr_: 'run' };
const human = (id: string, displayName: string): Actor => ({ kind: 'human', id, displayName });

export async function authenticate(db: D1Database, header: string | null): Promise<Who | null> {
  const token = header?.startsWith('Bearer ') ? header.slice(7).trim() : '';
  const kind = KIND[token.slice(0, 4)];
  if (kind === undefined) return null;
  const hash = await hashToken(token);
  const live = 'where t.hash = ? and t.kind = ? and t.revoked_at is null';
  if (kind === 'user') {
    const r = await db.prepare(`select u.id, u.name from tokens t join users u on u.id = t.subject ${live}`).bind(hash, kind).first<{ id: string; name: string }>();
    return r === null ? null : { kind, actor: human(r.id, r.name) };
  }
  if (kind === 'computer') {
    const r = await db.prepare(`select c.id as cid, o.id as uid, o.name as uname from tokens t join computers c on c.id = t.subject join users o on o.id = c.user_id ${live}`)
      .bind(hash, kind).first<{ cid: string; uid: string; uname: string }>();
    return r === null ? null : { kind, computerId: r.cid, owner: human(r.uid, r.uname) };
  }
  const r = await db.prepare(`select t.subject, t.pod from tokens t ${live}`).bind(hash, kind).first<{ subject: string; pod: string | null }>();
  return r === null || r.pod === null ? null : { kind, runId: r.subject, podId: r.pod };
}

async function mint(db: D1Database, prefix: string, kind: string, subject: string, pod: string | null): Promise<string> {
  const token = newToken(prefix);
  await db.prepare('insert into tokens (hash, kind, subject, pod) values (?, ?, ?, ?)').bind(await hashToken(token), kind, subject, pod).run();
  return token;
}

export async function createUser(db: D1Database, name: string): Promise<{ token: string; user: Actor }> {
  const id = newId('usr_');
  await db.prepare('insert into users (id, name) values (?, ?)').bind(id, name).run();
  return { token: await mint(db, 'jmu_', 'user', id, null), user: human(id, name) };
}

export async function createComputer(db: D1Database, userId: string, name: string, os: string, arch: string): Promise<{ computerId: string; token: string }> {
  const id = newId('cmp_');
  await db.prepare('insert into computers (id, user_id, name, os, arch) values (?, ?, ?, ?, ?)').bind(id, userId, name, os, arch).run();
  return { computerId: id, token: await mint(db, 'jmc_', 'computer', id, null) };
}

export const mintRunToken = (db: D1Database, runId: string, podId: string): Promise<string> => mint(db, 'jmr_', 'run', runId, podId);
export const dropRunToken = (db: D1Database, runId: string): Promise<unknown> => db.prepare("delete from tokens where kind = 'run' and subject = ?").bind(runId).run();
export async function podOfRun(db: D1Database, runId: string): Promise<string | null> {
  return (await db.prepare("select pod from tokens where kind = 'run' and subject = ?").bind(runId).first<{ pod: string }>())?.pod ?? null;
}

export const addPod = (db: D1Database, id: string): Promise<unknown> => db.prepare('insert into pods (id, created_at) values (?, ?)').bind(id, Date.now()).run();
export const dropPod = (db: D1Database, id: string): Promise<unknown> => db.prepare('delete from pods where id = ?').bind(id).run();
export async function podExists(db: D1Database, id: string): Promise<boolean> {
  return (await db.prepare('select 1 as one from pods where id = ?').bind(id).first()) !== null;
}
export async function allPodIds(db: D1Database): Promise<string[]> {
  return (await db.prepare('select id from pods').all<{ id: string }>()).results.map((r) => r.id);
}

/** An invite code is a join credential: only its SHA-256 is stored, like a token's. */
export const addInvite = (db: D1Database, hash: string, pod: string): Promise<unknown> => db.prepare('insert into invites (hash, pod) values (?, ?)').bind(hash, pod).run();
export const dropInvite = (db: D1Database, hash: string): Promise<unknown> => db.prepare('delete from invites where hash = ?').bind(hash).run();
export async function podOfInvite(db: D1Database, hash: string): Promise<string | null> {
  return (await db.prepare('select pod from invites where hash = ?').bind(hash).first<{ pod: string }>())?.pod ?? null;
}

export const addAgentPod = (db: D1Database, computerId: string, podId: string): Promise<unknown> =>
  db.prepare('insert or ignore into agent_pods (computer_id, pod_id) values (?, ?)').bind(computerId, podId).run();
export async function podsOfComputer(db: D1Database, computerId: string): Promise<string[]> {
  return (await db.prepare('select pod_id from agent_pods where computer_id = ?').bind(computerId).all<{ pod_id: string }>()).results.map((r) => r.pod_id);
}
