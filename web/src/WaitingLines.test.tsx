// The offline lines of /pod/:id: an agent a person @mentioned here whose computer is offline, and whose wake waits in the pod.
import { act, cleanup, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it } from 'vitest';
import type { Actor, AgentPresence, EventType, PresenceState } from '../../gen/ts/protocol';
import { App } from './App';
import { fakeHttp, fakeNet, frame, hello, note, you } from './testkit';

afterEach(cleanup);

const scout: Actor = { kind: 'agent', id: 'agt_1', displayName: 'Scout' };
const rex: Actor = { kind: 'agent', id: 'agt_2', displayName: 'Rex' };
const line = (name: string) => `${name} is offline. It will pick this up when its computer reconnects.`;
const presence = (agent: Actor, state: PresenceState): AgentPresence => ({ agent, harness: 'fake', computerId: 'cmp_1', state });
const said = (seq: number, body: string, author: Actor = you) => note(seq, { author, body, summary: body.slice(0, 120) });
const lines = () => Array.from(screen.queryByRole('list', { name: 'Waiting for a computer' })?.querySelectorAll('li') ?? [], (li) => li.textContent);

/** The pod page after a hello whose head is seq 2, so what arrives at seq 3 and on was posted while it was open. */
function open(agents: AgentPresence[]) {
  const net = fakeNet();
  render(<MemoryRouter initialEntries={['/pod/pod_1']}><App connect={() => net.connect} http={fakeHttp().http} me={you} /></MemoryRouter>);
  act(() => {
    net.open();
    net.deliver(frame('hello', undefined, hello({ headSeq: 2, agents })));
  });
  const deliver = (type: EventType, seq: number | undefined, data: unknown) => act(() => net.deliver(frame(type, seq, data)));
  return { net, deliver };
}

describe('an agent whose computer is offline', () => {
  it('says so when a person @mentions an agent that is offline', () => {
    const p = open([presence(scout, 'offline')]);
    expect(screen.queryByText(line('Scout'))).toBeNull();
    p.deliver('message.created', 3, said(3, '@Scout can you look at the auth fix?'));
    expect(screen.getByText(line('Scout'))).toBeTruthy();
  });

  it('drops the line when the agent comes back online', () => {
    const p = open([presence(scout, 'offline')]);
    p.deliver('message.created', 3, said(3, '@Scout look'));
    p.deliver('presence.changed', undefined, { actor: scout, state: 'online', computerId: 'cmp_1' });
    expect(screen.queryByText(line('Scout'))).toBeNull();
  });

  it('drops the line when a run of that agent starts, and the run line is then the only line about it', () => {
    const p = open([presence(scout, 'offline')]);
    p.deliver('message.created', 3, said(3, '@Scout look'));
    p.deliver('run.started', 4, { runId: 'run_1', agent: scout });
    expect(screen.queryByText(line('Scout'))).toBeNull();
    expect(screen.getByText('Scout is working on a run.')).toBeTruthy();
  });

  it('shows the line again if the computer drops before the agent takes the wake, and never once it has', () => {
    const p = open([presence(scout, 'offline')]);
    p.deliver('message.created', 3, said(3, '@Scout look'));
    p.deliver('presence.changed', undefined, { actor: scout, state: 'online', computerId: 'cmp_1' });
    p.deliver('presence.changed', undefined, { actor: scout, state: 'offline', computerId: 'cmp_1' });
    expect(screen.getByText(line('Scout'))).toBeTruthy();
    p.deliver('run.started', 4, { runId: 'run_1', agent: scout });
    p.deliver('run.ended', 5, { runId: 'run_1', agent: scout, outcome: 'completed' });
    expect(screen.queryByText(line('Scout'))).toBeNull();
  });

  it('shows no offline line for a mention of an agent that is online or working', () => {
    const p = open([presence(scout, 'online'), presence(rex, 'working')]);
    p.deliver('message.created', 3, said(3, '@Scout and @Rex, look'));
    expect(lines()).toEqual([]);
  });

  it('gives two offline agents two lines, and an agent named twice one line', () => {
    const p = open([presence(scout, 'offline'), presence(rex, 'offline')]);
    p.deliver('message.created', 3, said(3, '@Scout and @Rex, look. @Scout first.'));
    p.deliver('message.created', 4, said(4, '@Scout again'));
    expect(lines()).toEqual([line('Scout'), line('Rex')]);
  });

});

describe('which mentions give an offline line', () => {
  it('reads mentions by the pod\'s own rule: exact names, none inside code', () => {
    const p = open([presence(scout, 'offline'), presence(rex, 'offline')]);
    p.deliver('message.created', 3, said(3, 'ask `@Scout` later; @scout is not the name, and @Scouts is another'));
    expect(lines()).toEqual([]);
    p.deliver('message.created', 4, said(4, 'over to @Rex.'));
    expect(lines()).toEqual([line('Rex')]);
  });

  it('counts a person\'s message, not an agent\'s', () => {
    const p = open([presence(scout, 'offline'), presence(rex, 'online')]);
    p.deliver('message.created', 3, said(3, '@Scout over to you', rex));
    expect(lines()).toEqual([]);
  });

  it('counts only what was posted after the page\'s first hello, so a reload shows no line for an old mention', () => {
    const p = open([presence(scout, 'offline')]);
    p.deliver('message.created', 1, said(1, '@Scout from yesterday'));
    p.deliver('message.created', 2, said(2, '@Scout still waiting'));
    expect(lines()).toEqual([]);
  });

  it('keeps a line across a dropped socket, and counts a mention posted while the page was away', () => {
    const p = open([presence(scout, 'offline'), presence(rex, 'offline')]);
    p.deliver('message.created', 3, said(3, '@Scout look'));
    act(() => p.net.drop());
    act(() => {
      p.net.open();
      p.net.deliver(frame('hello', undefined, hello({ headSeq: 4, agents: [presence(scout, 'offline'), presence(rex, 'offline')] })));
    });
    p.deliver('message.created', 4, said(4, '@Rex look'));
    expect(lines()).toEqual([line('Scout'), line('Rex')]);
  });
});
