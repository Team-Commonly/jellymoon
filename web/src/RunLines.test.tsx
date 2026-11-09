// The run lines of /pod/:id: a going run with a Stop button for those who may stop it, and the quiet line it leaves.
import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { App } from './App';
import { ApiError } from './http';
import { fakeHttp, fakeNet, frame, hello, other, you } from './testkit';

afterEach(cleanup);

const scout = { kind: 'agent' as const, id: 'agt_1', displayName: 'Scout' };
const sage = { kind: 'agent' as const, id: 'agt_2', displayName: 'Sage' };

function mount(opts: { as?: typeof you; stop?: (podId: string, runId: string) => Promise<void> } = {}) {
  const net = fakeNet();
  const h = fakeHttp(undefined, undefined, undefined, opts.stop);
  const as = opts.as ?? you;
  const members = [{ actor: you, role: 'owner' }, { actor: other, role: 'member' }] as const;
  const join = () => act(() => {
    net.open();
    net.deliver(frame('hello', undefined, hello({ you: as, members: [...members] })));
  });
  render(<MemoryRouter initialEntries={['/pod/pod_1']}><App connect={() => net.connect} http={h.http} signedIn /></MemoryRouter>);
  join();
  const seq = { n: 0 };
  const started = (runId: string, agent = scout, at = ++seq.n) => act(() => net.deliver(frame('run.started', at, { runId, agent })));
  const ended = (runId: string, outcome: string, agent = scout, at = ++seq.n) => act(() => net.deliver(frame('run.ended', at, { runId, agent, outcome })));
  return { net, http: h, started, ended, join, seq };
}

const lines = () => within(screen.getByRole('list', { name: 'Runs' })).queryAllByRole('listitem');
const stopButton = (name = 'Scout') => screen.getByRole('button', { name: `Stop ${name}'s run` }) as HTMLButtonElement;

describe('a going run', () => {
  it('shows "<agent> is working on a run." with a Stop button for a pod owner', () => {
    const m = mount();
    m.started('run_1');
    expect(lines().map((l) => l.textContent)).toEqual(['Scout is working on a run.Stop']);
    expect(lines()[0].getAttribute('data-state')).toBe('going');
    expect(stopButton().disabled).toBe(false);
  });

  it('shows the line without Stop to a plain member, who the route would refuse', () => {
    const m = mount({ as: other });
    m.started('run_1');
    expect(lines().map((l) => l.textContent)).toEqual(['Scout is working on a run.']);
    expect(screen.queryByRole('button', { name: /^Stop/ })).toBeNull();
  });

  it('shows one line per going run, each with its own Stop', () => {
    const m = mount();
    m.started('run_1');
    m.started('run_2', sage);
    expect(lines()).toHaveLength(2);
    expect(stopButton('Scout')).toBeTruthy();
    expect(stopButton('Sage')).toBeTruthy();
  });

  it('shows nothing when no run has started', () => {
    mount();
    expect(screen.queryByRole('list', { name: 'Runs' })).toBeNull();
  });
});

describe('pressing Stop', () => {
  it('posts the stop for that run, disables the button while the request is out, and leaves the line going until run.ended', async () => {
    let answer!: () => void;
    const m = mount({ stop: () => new Promise<void>((r) => { answer = r; }) });
    m.started('run_1');
    fireEvent.click(stopButton());
    expect(m.http.stopped).toEqual(['pod_1/run_1']);
    expect(stopButton().disabled).toBe(true);
    await act(async () => answer());
    expect(lines()[0].getAttribute('data-state')).toBe('going');
    expect(lines()[0].textContent).toContain('Scout is working on a run.');
    expect(stopButton().disabled).toBe(true);
    m.ended('run_1', 'cancelled');
    expect(lines().map((l) => l.textContent)).toEqual(["Scout's run stopped."]);
    expect(screen.queryByRole('button', { name: /^Stop/ })).toBeNull();
  });

  it('shows the server\'s message inline on a refusal, re-enables Stop and keeps the run going', async () => {
    const m = mount({ stop: async () => { throw new ApiError(403, 'Only the agent\'s owner or a pod owner may stop a run.'); } });
    m.started('run_1');
    await act(async () => fireEvent.click(stopButton()));
    expect(within(lines()[0]).getByRole('alert').textContent).toBe('Only the agent\'s owner or a pod owner may stop a run.');
    expect(stopButton().disabled).toBe(false);
    expect(lines()[0].getAttribute('data-state')).toBe('going');
  });

  it('says it could not reach the server when the request itself fails, and clears the note on the next press', async () => {
    let fail = true;
    const m = mount({ stop: async () => { if (fail) throw new TypeError('network'); } });
    m.started('run_1');
    await act(async () => fireEvent.click(stopButton()));
    expect(within(lines()[0]).getByRole('alert').textContent).toMatch(/could not reach/i);
    fail = false;
    await act(async () => fireEvent.click(stopButton()));
    expect(within(lines()[0]).queryByRole('alert')).toBeNull();
  });
});

describe('an ended run', () => {
  it.each([
    ['cancelled', "Scout's run stopped."],
    ['completed', "Scout's run finished."],
    ['failed', "Scout's run failed."],
    ['expired', "Scout's run timed out."],
  ])('says %s as one quiet line with no Stop', (outcome, text) => {
    const m = mount();
    m.started('run_1');
    m.ended('run_1', outcome);
    expect(lines().map((l) => l.textContent)).toEqual([text]);
    expect(lines()[0].getAttribute('data-state')).toBe('ended');
    expect(screen.queryByRole('button', { name: /^Stop/ })).toBeNull();
  });

  it('never shows an agent as both working and ended: its next run replaces the old line', () => {
    const m = mount();
    m.started('run_1');
    m.ended('run_1', 'completed');
    m.started('run_2');
    expect(lines().map((l) => l.getAttribute('data-state'))).toEqual(['going']);
  });

  it('keeps another agent\'s ended line beside a going run', () => {
    const m = mount();
    m.started('run_1', sage);
    m.ended('run_1', 'failed', sage);
    m.started('run_2');
    expect(lines().map((l) => l.textContent)).toEqual(["Sage's run failed.", 'Scout is working on a run.Stop']);
  });
});

describe('after a reconnect', () => {
  it('keeps the going run, ignores the replayed start, and shows the end that came while away', () => {
    vi.useFakeTimers();
    try {
      const m = mount();
      m.started('run_1');
      act(() => m.net.drop());
      expect(lines()[0].getAttribute('data-state')).toBe('going');
      act(() => void vi.advanceTimersByTime(1000));
      m.join();
      expect(m.net.sent()[0]).toMatchObject({ type: 'subscribe', data: { sinceSeq: 1 } });
      m.started('run_1', scout, 1);
      expect(lines()[0].getAttribute('data-state')).toBe('going');
      m.ended('run_1', 'cancelled', scout, 2);
      expect(lines().map((l) => l.textContent)).toEqual(["Scout's run stopped."]);
    } finally {
      vi.useRealTimers();
    }
  });
});
