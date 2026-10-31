import { describe, expect, test } from 'bun:test';
import { agentPresence, isoOf, nextDeadline } from '../src';
import type { Cmd, Out, PodState } from '../src';
import { AG1, AG3, CMP_1, CMP_2, U_AGENT_OWNER, id, play, world } from './helpers';

const AG2 = id('agt', 2);
/** The helper world has a live run for each agent; presence is clearest with none. */
const quiet = (): PodState => ({ ...world(), runs: {} });
const online = (c: string): Cmd => ({ type: 'computer_online', computerId: c });
const offline = (c: string): Cmd => ({ type: 'computer_offline', computerId: c });
const beat = (c: string): Cmd => ({ type: 'heartbeat', computerId: c });
const live = (out: Out[]) => out.flatMap((o) => (o.kind === 'live' ? [{ who: o.event.data.actor.id, state: o.event.data.state, cmp: o.event.data.computerId, ts: o.event.ts, seq: o.event.seq }] : []));

describe('presence', () => {
  test('a computer coming online tells the pod about each agent it hosts, with no seq', () => {
    const r = play([[online(CMP_1), 1000]], quiet());
    expect(live(r.out)).toEqual([AG1, AG2].map((who) => ({ who, state: 'online', cmp: CMP_1, ts: isoOf(1000), seq: undefined })));
    expect(r.state.seq).toBe(0);
    expect(r.out.some((o) => o.kind === 'event')).toBe(false);
  });

  test('online twice, or offline twice, says nothing the second time', () => {
    const r = play([[online(CMP_1), 1000], [online(CMP_1), 2000], [offline(CMP_1), 3000], [offline(CMP_1), 4000]], quiet());
    expect(live(r.out).map((e) => e.state)).toEqual(['online', 'online', 'offline', 'offline']);
  });

  test('a computer is online until 45 s after its last sign of life, to the millisecond', () => {
    const base = play([[online(CMP_1), 1000]], quiet());
    expect(nextDeadline(base.state)).toBe(46_000);
    expect(live(play([[{ type: 'tick' }, 45_999]], base.state).out)).toEqual([]);
    const late = play([[{ type: 'tick' }, 90_000]], base.state);
    expect(live(late.out).map((e) => [e.state, e.ts])).toEqual([['offline', isoOf(46_000)], ['offline', isoOf(46_000)]]);
    expect(agentPresence(late.state, AG1)).toBe('offline');
  });

  test('a heartbeat is a sign of life, and from an offline computer it brings the computer back', () => {
    const r = play([[online(CMP_1), 0], [beat(CMP_1), 30_000], [{ type: 'tick' }, 74_999]], quiet());
    expect(live(r.out).map((e) => e.state)).toEqual(['online', 'online']);
    const gone = play([[{ type: 'tick' }, 75_000], [beat(CMP_1), 80_000]], r.state);
    expect(live(gone.out).map((e) => e.state)).toEqual(['offline', 'offline', 'online', 'online']);
  });

  test('an agent with an active run shows working, whatever its computer does', () => {
    const w = { ...world(), runs: { run: world().runs[Object.keys(world().runs)[0]] } };
    const r = play([[online(CMP_1), 0], [offline(CMP_1), 1000]], w);
    expect(live(r.out).map((e) => e.who)).toEqual([AG2, AG2]);
    expect(agentPresence(r.state, AG1)).toBe('working');
  });

  test('registering an agent on an online computer announces it; online survives a restart', () => {
    const up = play([[online(CMP_2), 0]], quiet());
    const saved = JSON.parse(JSON.stringify(up.state)) as PodState;
    const reg: Cmd = { type: 'agent_register', agentId: id('agt', 7), ownerId: U_AGENT_OWNER.id, computerId: CMP_2, agentName: 'late' };
    expect(live(play([[reg, 500]], saved).out)).toEqual([{ who: id('agt', 7), state: 'online', cmp: CMP_2, ts: isoOf(500), seq: undefined }]);
    expect(agentPresence(saved, AG3)).toBe('online');
  });

  test('a pod saved before presence existed reads as everything offline', () => {
    const old = quiet();
    delete old.online;
    expect(agentPresence(old, AG1)).toBe('offline');
    expect(nextDeadline(old)).toBeNull();
  });
});
