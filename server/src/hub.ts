// Every pod in the process, the directory (users, computers), token lookup, the clock and the one timer.
import type { Actor } from '../../gen/ts/protocol';
import { emptyPod } from '../../podcore/src/index';
import type { Db, Tx } from './db';
import { openDb } from './db';
import type { DbEnv } from './db';
import { hashToken, newId, newToken } from './ids';
import { PodRuntime } from './pod';
import type { Reply } from './pod';
import { allComputers, allUsers, claimGithubUser, findToken, githubUserId, loadPods, migrate, saveComputer, saveToken, saveUser, settleUnrecorded, sweepSignIn } from './store';
import type { ComputerRow, UserRow } from './store';

export type Principal =
  | { kind: 'user'; actor: Actor }
  | { kind: 'computer'; computer: ComputerRow; owner: Actor }
  | { kind: 'run'; runId: string; podId: string };

const PREFIX: Record<string, 'user' | 'computer' | 'run'> = { jmu_: 'user', jmc_: 'computer', jmr_: 'run' };
const MAX_TIMER = 2 ** 31 - 1;
/** A user token's life (ADR 0008): DeviceTokenResponse.expiresAt and the session cookie's Max-Age say the same. */
export const USER_TOKEN_MS = 30 * 86_400_000;
/** The wait before a failed tick is tried again, so a database that is down is not hammered. */
const RETRY_MS = 500;

/** Another process holds the database's boot lock; main.ts exits with its message. */
export class BootRefused extends Error {}

const actorOf = (u: UserRow): Actor => ({ kind: 'human', id: u.id, displayName: u.name });

export class Hub {
  readonly pods = new Map<string, PodRuntime>();
  readonly users = new Map<string, UserRow>();
  readonly computers = new Map<string, ComputerRow>();
  /** run id -> pod id, and computer id -> the pods where it hosts an agent; both rebuilt from pod state at boot. */
  readonly runPod = new Map<string, string>();
  readonly computerPods = new Map<string, Set<string>>();
  /** Token hash -> a close for each pod socket it opened; logout closes them. */
  readonly sockets = new Map<string, Set<() => void>>();
  /** Set by /__test/clock: from then on the clock only moves by another call. */
  private fixed: number | null = null;
  private timer: ReturnType<typeof setTimeout> | undefined;

  private constructor(readonly db: Db) {}

  /** Restores every pod, applies `boot {staleMs: 0}` (every command is persisted, heartbeats too) and arms the timer. */
  static async open(env: DbEnv, lost: (why: string) => void): Promise<Hub> {
    return Hub.boot(await openDb(env), lost);
  }

  /**
   * Takes the database's boot lock before any statement (ADR 0010): while another process holds it this one restores nothing
   * and throws BootRefused, so it never serves. `lost` is called if the lock goes later.
   */
  static async boot(db: Db, lost: (why: string) => void): Promise<Hub> {
    const refusal = await db.bootLock?.(lost);
    if (refusal !== undefined) throw new BootRefused(refusal);
    return Hub.restore(db);
  }

  /**
   * A restart over storage that already exists; tests call it twice on one Db to prove state survives. A connector write the
   * last process released and never heard back on is settled as unknown here, and never sent again (ADR 0009).
   */
  static async restore(db: Db): Promise<Hub> {
    await migrate(db);
    const hub = new Hub(db);
    await settleUnrecorded(db, hub.now());
    await sweepSignIn(db, hub.now());
    for (const u of await allUsers(db)) hub.users.set(u.id, u);
    for (const c of await allComputers(db)) hub.computers.set(c.id, c);
    for (const row of await loadPods(db)) {
      const pod = hub.runtime(row.id, row.state, row.meta, true);
      hub.pods.set(row.id, pod);
      hub.indexPod(pod);
    }
    await Promise.all([...hub.pods.values()].map((p) => hub.wake(p)));
    return hub;
  }

  /**
   * Applies `boot {staleMs: 0}` once, with now. The sockets are the adapter's truth about who is connected and a restarted
   * process has none, so every computer the pod still holds online is then set offline.
   */
  private async wake(p: PodRuntime): Promise<void> {
    await p.exec({ type: 'boot', staleMs: 0 });
    for (const id of Object.keys(p.state.online ?? {})) await p.exec({ type: 'computer_offline', computerId: id });
  }

  now = (): number => this.fixed ?? Date.now();

  private runtime(id: string, state: PodRuntime['state'], meta: PodRuntime['meta'], persisted: boolean): PodRuntime {
    return new PodRuntime(id, state, meta, { db: this.db, now: () => this.now(), rearm: () => this.rearm() }, persisted);
  }

  indexPod(pod: PodRuntime): void {
    for (const a of Object.values(pod.state.agents)) {
      const set = this.computerPods.get(a.computerId) ?? new Set<string>();
      this.computerPods.set(a.computerId, set.add(pod.id));
    }
    for (const id of Object.keys(pod.state.runs)) this.runPod.set(id, pod.id);
  }

  async createPod(owner: Actor, name: string): Promise<{ pod: PodRuntime; reply: Reply | undefined }> {
    const id = newId('pod_');
    const pod = this.runtime(id, emptyPod(id), { name, createdAt: new Date(this.now()).toISOString(), harness: {} }, false);
    const reply = await pod.exec({ type: 'pod_create', ownerId: owner.id, ownerName: owner.displayName });
    if (reply?.ok) this.pods.set(id, pod);
    return { pod, reply };
  }

  async createUser(name: string): Promise<{ token: string; user: Actor }> {
    const u: UserRow = { id: newId('usr_'), name };
    const token = await this.db.tx(async (t) => { await saveUser(t, u); return this.mint(t, 'jmu_', 'user', u.id, this.now() + USER_TOKEN_MS); });
    this.users.set(u.id, u);
    return { token, user: actorOf(u) };
  }

  /** The user a GitHub account signs in as, keyed by GitHub's numeric id (a login can be renamed); created on first sign-in. */
  async githubUser(githubId: string, login: string): Promise<Actor> {
    const known = this.users.get((await githubUserId(this.db, githubId)) ?? '');
    if (known !== undefined) return actorOf(known);
    const u = await this.db.tx((t) => claimGithubUser(t, githubId, { id: newId('usr_'), name: login }));
    this.users.set(u.id, u);
    return actorOf(u);
  }

  /** A new jmu_ for the user, and when it expires. */
  async userToken(userId: string): Promise<{ token: string; expiresAt: number }> {
    const expiresAt = this.now() + USER_TOKEN_MS;
    return { token: await this.db.tx((t) => this.mint(t, 'jmu_', 'user', userId, expiresAt)), expiresAt };
  }

  /** Registers an open pod socket's close under its token's hash. */
  addSocket(hash: string, close: () => void): void {
    this.sockets.set(hash, (this.sockets.get(hash) ?? new Set()).add(close));
  }

  /** A pod socket closed; a hash whose last socket closed leaves the map, so it holds only tokens with an open socket. */
  dropSocket(hash: string, close: () => void): void {
    const set = this.sockets.get(hash);
    if (set?.delete(close) && set.size === 0) this.sockets.delete(hash);
  }

  /** Closes every pod socket opened with the token whose hash this is. */
  closeSockets(hash: string): void {
    for (const close of this.sockets.get(hash) ?? []) close();
    this.sockets.delete(hash);
  }

  async createComputer(userId: string, name: string, os: string, arch: string): Promise<{ computerId: string; token: string }> {
    const c: ComputerRow = { id: newId('cmp_'), userId, name, os, arch };
    const token = await this.db.tx(async (t) => { await saveComputer(t, c); return this.mint(t, 'jmc_', 'computer', c.id); });
    this.computers.set(c.id, c);
    return { computerId: c.id, token };
  }

  private async mint(t: Tx, prefix: string, kind: string, subject: string, expiresAt?: number): Promise<string> {
    const token = newToken(prefix);
    await saveToken(t, hashToken(token), kind, subject, undefined, expiresAt);
    return token;
  }

  async authenticate(header: string | undefined): Promise<Principal | null> {
    const token = header?.startsWith('Bearer ') ? header.slice(7).trim() : '';
    const kind = PREFIX[token.slice(0, 4)];
    const row = kind === undefined ? undefined : await findToken(this.db, hashToken(token), this.now());
    if (row === undefined || row.kind !== kind) return null;
    if (kind === 'run') return row.pod === null ? null : { kind, runId: row.subject, podId: row.pod };
    if (kind === 'user') {
      const u = this.users.get(row.subject);
      return u === undefined ? null : { kind, actor: actorOf(u) };
    }
    const computer = this.computers.get(row.subject);
    const owner = computer === undefined ? undefined : this.users.get(computer.userId);
    return computer === undefined || owner === undefined ? null : { kind, computer, owner: actorOf(owner) };
  }

  /** Moves the test clock and applies every tick that is now due before returning. */
  async setClock(now: number): Promise<void> {
    this.fixed = now;
    await this.tickDue();
  }

  private async tickDue(): Promise<void> {
    const now = this.now();
    const due = [...this.pods.values()].filter((p) => p.deadline !== null && p.deadline <= now);
    let failed = false;
    try {
      await sweepSignIn(this.db, now).catch((e) => console.error('sign-in sweep failed', e));
      const done = await Promise.allSettled(due.map((p) => p.exec({ type: 'tick' })));
      done.forEach((d, i) => {
        if (d.status === 'fulfilled') return;
        failed = true;
        console.error('tick failed for pod', due[i].id, d.reason);
      });
    } finally {
      // A pod whose write failed keeps its old deadline, so the re-armed timer retries it. The timer must never end up unarmed.
      this.rearm(failed ? RETRY_MS : 0);
    }
  }

  /** One timer, armed at the earliest deadline of any pod; re-armed after every command. A fixed clock never fires it. `least` is the shortest wait. */
  rearm(least = 0): void {
    clearTimeout(this.timer);
    this.timer = undefined;
    if (this.fixed !== null) return;
    let next: number | null = null;
    for (const p of this.pods.values()) if (p.deadline !== null && (next === null || p.deadline < next)) next = p.deadline;
    if (next === null) return;
    this.timer = setTimeout(() => { this.timer = undefined; this.tickDue().catch((e) => console.error('tick failed', e)); }, Math.min(Math.max(least, next - this.now()), MAX_TIMER));
  }
}
