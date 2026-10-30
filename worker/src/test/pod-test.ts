// The pod object with a settable clock and write counters, for local runs and previews. The production bundle has the plain PodDO.
// The clock lives in the object's own storage ('clock': milliseconds, or 'none'). A fixed clock never arms the alarm; /_in/clock
// runs any due tick itself, so expiry fires with zero other traffic. A new object asks D1 once for a clock set before it existed.
import { PodDO } from '../pod-do';
import { parse } from '../wire';

export class TestPodDO extends PodDO {
  private declare fixed: number | null | undefined;
  private declare writes: Map<string, { commands: number; rowsWritten: number }> | undefined;

  private setFixed(ms: number | null): void {
    this.fixed = ms;
    this.sql.exec('insert or replace into kv (k, v) values (?, ?)', 'clock', ms === null ? 'none' : String(ms));
  }

  private fixedNow(): number | null {
    if (this.fixed === undefined) {
      const [row] = this.sql.exec<{ v: string }>("select v from kv where k = 'clock'").toArray();
      this.fixed = row === undefined || row.v === 'none' ? null : Number(row.v);
    }
    return this.fixed;
  }

  override now(): number { return this.fixedNow() ?? Date.now(); }

  protected override alarmAt(): number | null { return this.fixedNow() === null ? super.alarmAt() : null; }

  protected override note(type: string, rowsWritten: number): void {
    this.writes ??= new Map();
    const w = this.writes.get(type) ?? { commands: 0, rowsWritten: 0 };
    this.writes.set(type, { commands: w.commands + 1, rowsWritten: w.rowsWritten + rowsWritten });
  }

  override alarm(): void {
    this.sql.exec("insert or replace into kv (k, v) values ('alarm_fired', ?)", String(Date.now()));
    super.alarm();
  }

  override async fetch(req: Request): Promise<Response> {
    const path = new URL(req.url).pathname;
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
