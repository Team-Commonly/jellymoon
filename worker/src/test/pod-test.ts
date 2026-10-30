// The pod object with a settable clock and write counters, for local runs and previews. The production bundle has the plain PodDO.
// The clock lives in the object's own storage ('clock': this process's id and milliseconds, or 'none'); a restarted server is a
// new process and so starts on its real clock again, as the contract says. A fixed clock stores the object's own deadline alarm,
// a far-future real time that never fires by itself (the suite reads it back to prove it); /_in/clock runs any due tick itself, so
// expiry fires with zero other traffic. A new object asks D1 once for a clock set before it existed.
import { PodDO } from '../pod-do';
import { parse } from '../wire';

/** One per server process (a module is evaluated once per start). A clock set by an earlier process is not this one's. */
let proc: string | undefined;
/** Random values are not allowed in global scope, so the id is made on first use. */
const processId = (): string => (proc ??= crypto.randomUUID());

export class TestPodDO extends PodDO {
  /** Minted per instance: a different value after an eviction proves the runtime built a new object. */
  private readonly instance = crypto.randomUUID();
  private requests = 0;
  private declare fixed: number | null | undefined;
  private declare writes: Map<string, { commands: number; rowsWritten: number }> | undefined;

  private setFixed(ms: number | null): void {
    this.fixed = ms;
    this.sql.exec('insert or replace into kv (k, v) values (?, ?)', 'clock', ms === null ? 'none' : `${processId()}:${ms}`);
  }

  private fixedNow(): number | null {
    if (this.fixed === undefined) {
      const [row] = this.sql.exec<{ v: string }>("select v from kv where k = 'clock'").toArray();
      this.fixed = row === undefined || !row.v.startsWith(`${processId()}:`) ? null : Number(row.v.slice(processId().length + 1));
    }
    return this.fixed;
  }

  /** Drops every in-memory field and runs the restore path, as the constructor does when a hibernated object wakes. The sockets stay attached. */
  private evict(): void {
    this.fixed = undefined;
    this.writes = undefined;
    this.reload();
  }

  override now(): number { return this.fixedNow() ?? Date.now(); }

  /**
   * The production value, untouched: a fixed deadline is a far-future real time, so the stored alarm never fires on its own.
   * Two accommodations, neither of which changes a deadline that is still ahead: a discard that moved the stored alarm to fire
   * keeps it armed until it has run, and a fixed deadline already past in real time is not stored, or it would re-fire for ever.
   */
  protected override alarmAt(): number | null {
    const at = super.alarmAt();
    if (this.fixedNow() === null || at === null) return at;
    const [row] = this.sql.exec<{ v: string }>("select v from kv where k = 'alarm_at'").toArray();
    if (row !== undefined) return Number(row.v);
    return at > Date.now() ? at : null;
  }

  protected override note(type: string, rowsWritten: number): void {
    this.writes ??= new Map();
    const w = this.writes.get(type) ?? { commands: 0, rowsWritten: 0 };
    this.writes.set(type, { commands: w.commands + 1, rowsWritten: w.rowsWritten + rowsWritten });
  }

  /**
   * The runtime throws this object away: ends in-flight requests, drops sockets, and builds a new instance on the next request or
   * alarm. With alarmMs, the alarm the object itself stored is moved to fire that many real ms ahead (nothing new is planted: there
   * must be a stored alarm, and its old time is recorded and answered), so the runtime fires the object's own alarm on the new instance.
   */
  private async discard(req: Request): Promise<Response> {
    const b = await parse(req);
    const now = b?.now;
    if (now !== undefined && (typeof now !== 'number' || !Number.isInteger(now) || now < 0)) return new Response('bad', { status: 400 });
    const stored = await this.ctx.storage.getAlarm();
    if (typeof b?.alarmMs === 'number' && stored === null) return new Response('the object has no stored alarm to fire', { status: 409 });
    if (now !== undefined) this.setFixed(now); // the clock moves with no tick: nothing runs until the object is woken
    if (typeof b?.alarmMs === 'number' && stored !== null) {
      const at = Date.now() + b.alarmMs;
      this.sql.exec("insert or replace into kv (k, v) values ('alarm_at', ?), ('alarm_was', ?)", String(at), String(stored));
      // The clock the server reads once the alarm runs, so the alarm's tick, not the restore, finds the deadline due.
      if (typeof b.alarmNow === 'number') this.sql.exec("insert or replace into kv (k, v) values ('alarm_clock', ?)", String(b.alarmNow));
      await this.ctx.storage.setAlarm(at);
    }
    setTimeout(() => this.ctx.abort('discarded by the test entry'), 0);
    return Response.json({ discarded: this.instance, alarm: stored });
  }

  override alarm(): void {
    const [due] = this.sql.exec<{ v: string }>("select v from kv where k = 'alarm_clock'").toArray();
    if (due !== undefined) { this.setFixed(Number(due.v)); this.sql.exec("delete from kv where k = 'alarm_clock'"); }
    const [was] = this.sql.exec<{ v: string }>("select v from kv where k = 'alarm_was'").toArray();
    // Consumed: once it has run, the alarm is the production one again, so it cannot re-fire for ever.
    this.sql.exec("delete from kv where k in ('alarm_at', 'alarm_was')");
    this.sql.exec("insert or replace into kv (k, v) values ('alarm_fired', ?)", String(Date.now()));
    // D1 outlives the object, so the suite reads what woke it without sending the object a request.
    this.ctx.waitUntil(this.env.DB.prepare("insert or replace into kv (k, v) values (?, ?)").bind(`woke:${this.id}`, JSON.stringify({ instance: this.instance, requests: this.requests, was: was === undefined ? null : Number(was.v) })).run());
    super.alarm();
  }

  override async fetch(req: Request): Promise<Response> {
    const path = new URL(req.url).pathname;
    if (path === '/_in/instance') return Response.json({ instance: this.instance, requests: this.requests });
    if (path === '/_in/stored-alarm') return Response.json({ at: await this.ctx.storage.getAlarm() }); // what the object itself stored
    this.requests += 1;
    if (path === '/_in/discard') return this.discard(req);
    if (path === '/_in/evict') { this.evict(); return Response.json({ evicted: true }); }
    if (path === '/_in/stats') return Response.json(Object.fromEntries(this.writes ?? []));
    if (path === '/_in/alarm-fired') return Response.json({ at: this.sql.exec<{ v: string }>("select v from kv where k = 'alarm_fired'").toArray()[0]?.v ?? null });
    if (path === '/_in/clock') {
      const now = (await parse(req))?.now;
      if (typeof now !== 'number' || !Number.isInteger(now) || now < 0) return new Response('bad', { status: 400 });
      this.setFixed(now);
      this.exec({ type: 'tick' });
      return Response.json({ now });
    }
    const [seen] = this.sql.exec("select 1 as one from kv where k = 'clock'").toArray();
    if (seen === undefined) {
      const row = await this.env.DB.prepare("select v from kv where k = 'clock'").first<{ v: string }>();
      this.setFixed(row === null ? null : Number(row.v));
    }
    return super.fetch(req);
  }
}
