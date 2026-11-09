// /pod/:id at component level: a fake socket in, frames out. No browser, no network.
import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it } from 'vitest';
import { App } from './App';
import type { Message } from '../../gen/ts/protocol';
import { ask, fakeHttp, fakeNet, frame, hello, note, other, you } from './testkit';

afterEach(cleanup);

const theirs = (seq: number, over: Partial<Message> = {}) => ask(seq, { needs: { to: { userIds: [other.id] }, action: 'decide' }, ...over });

function mount(msgs: Message[] = [], path = '/pod/pod_1') {
  const net = fakeNet();
  const tree = (key: number) => <MemoryRouter initialEntries={[path]}><App key={key} connect={() => net.connect} http={fakeHttp().http} signedIn /></MemoryRouter>;
  const view = render(tree(1));
  const push = (m: Message) => act(() => net.deliver(frame('message.created', m.seq, m)));
  act(() => {
    net.open();
    net.deliver(frame('hello', undefined, hello()));
  });
  msgs.forEach(push);
  const seq = { n: 100 };
  const rulingOf = (m: Message, optionId: 'a' | 'b' | 'c', by = you) => act(() => net.deliver(frame('message.updated', ++seq.n, {
    id: m.id, decision: { id: m.decision!.id, state: 'ruled', ruling: { optionId, by, at: '2026-11-08T11:00:00Z' } },
  })));
  const remount = () => {
    view.rerender(tree(2));
    act(() => {
      net.open();
      net.deliver(frame('hello', undefined, hello()));
    });
    msgs.forEach(push);
  };
  return { net, push, rulingOf, remount };
}

const press = (key: string, target: Element = document.body, init: object = {}) => fireEvent.keyDown(target, { key, ...init });
const cards = () => screen.getAllByRole('article');
const rules = (net: ReturnType<typeof fakeNet>) => net.sent().filter((f) => f.type === 'decision.rule');
const composer = () => screen.getByRole('textbox', { name: 'Message' });
const toggle = () => screen.getByRole('button', { name: /Needs you/ });

describe('the stream', () => {
  it('shows a card per message, oldest first and newest at the bottom, with author, headline and status', () => {
    mount([note(1, { summary: 'First', status: 'done' }), note(2, { summary: 'Second', status: 'in_progress' })]);
    const list = cards();
    expect(list.map((c) => within(c).getByRole('heading', { level: 3 }).textContent)).toEqual(['First', 'Second']);
    expect(within(list[0]).getByText('Scout')).toBeTruthy();
    expect(within(list[0]).getByText('done')).toBeTruthy();
    expect(within(list[1]).getByText('in_progress')).toBeTruthy();
  });

  it('shows no status chip on a person\'s message', () => {
    mount([note(1, { author: you, status: 'done' })]);
    expect(within(cards()[0]).queryByText('done')).toBeNull();
  });

  it('shows the receipt chip only when the message has one', () => {
    const receipt = { runId: 'r', agentId: 'a', computerId: 'c', harness: 'codex' as const, startedAt: 's', postedAt: 'p', toolCalls: 7, wakeReason: 'mention' as const };
    mount([note(1, { receipt }), note(2)]);
    expect(within(cards()[0]).getByText('codex · 7 tool calls')).toBeTruthy();
    expect(within(cards()[1]).queryByText(/tool calls/)).toBeNull();
  });

  it('treats progress and fyi as quiet and ask, result and error as loud', () => {
    mount([note(1, { kind: 'progress' }), note(2, { kind: 'fyi' }), note(3, { kind: 'result' }), note(4, { kind: 'error', status: 'failed' }), ask(5)]);
    expect(cards().map((c) => c.getAttribute('data-quiet'))).toEqual(['true', 'true', 'false', 'false', 'false']);
    expect(cards().map((c) => c.getAttribute('data-kind'))).toEqual(['progress', 'fyi', 'result', 'error', 'ask']);
  });

  it('folds the body, opens it on click and on Enter, and folds it again', () => {
    mount([note(1, { body: 'The long detail' })]);
    expect(screen.queryByText('The long detail')).toBeNull();
    fireEvent.click(screen.getByRole('heading', { name: 'Note 1' }));
    expect(screen.getByText('The long detail')).toBeTruthy();
    press('Enter');
    expect(screen.queryByText('The long detail')).toBeNull();
    press('Enter');
    expect(screen.getByText('The long detail')).toBeTruthy();
  });

  it('shows an ask that arrives over the socket without a reload', () => {
    const m = mount();
    expect(screen.queryByText('Ask 1')).toBeNull();
    m.push(ask(1));
    expect(screen.getByText('Ask 1')).toBeTruthy();
  });

  it('renders a 120-character summary and a 40-character option label whole', () => {
    const summary = 'S'.repeat(60) + ' ' + 'T'.repeat(59);
    const label = 'L'.repeat(39) + 'Z';
    const base = ask(1);
    mount([ask(1, { summary, options: [{ id: 'a', label, consequence: 'x' }, ...base.options!.slice(1)] })]);
    expect(summary).toHaveLength(120);
    expect(screen.getByRole('heading', { name: summary })).toBeTruthy();
    expect(screen.getByText(new RegExp(label))).toBeTruthy();
  });
});

describe('an ask card', () => {
  it('shows what happened, what was checked, the stakes and the recommendation', () => {
    mount([ask(1)]);
    const card = cards()[0];
    expect(within(card).getByText('Tests passed')).toBeTruthy();
    expect(within(card).getByText('Lint is clean')).toBeTruthy();
    expect(within(card).getByText('Deploy at noon')).toBeTruthy();
    expect(within(card).getByText('Safest')).toBeTruthy();
  });

  it('says "Nothing was checked" when the checked list is empty', () => {
    const base = ask(1);
    mount([ask(1, { context: { ...base.context!, checked: [] } })]);
    expect(screen.getByText('Nothing was checked')).toBeTruthy();
  });

  it('offers options as numbered buttons and marks the recommended one', () => {
    mount([ask(1)]);
    const buttons = within(cards()[0]).getAllByRole('button', { name: /Option/ });
    expect(buttons.map((b) => b.textContent)).toEqual(['1 Option a', '2 Option b Recommended', '3 Option c']);
  });

  it('shows an option consequence on focus and on hover, and hides it again', () => {
    mount([ask(1)]);
    const b = screen.getByRole('button', { name: /Option b/ });
    expect(screen.queryByText('Does b')).toBeNull();
    fireEvent.focus(b);
    expect(screen.getByText('Does b')).toBeTruthy();
    fireEvent.blur(b);
    expect(screen.queryByText('Does b')).toBeNull();
    fireEvent.mouseEnter(screen.getByRole('button', { name: /Option c/ }));
    expect(screen.getByText('Does c')).toBeTruthy();
  });

  it('shows the error frame on the card whose ruling was refused', () => {
    const m = mount([ask(1)]);
    press('a');
    press('Enter');
    act(() => m.net.deliver(frame('error', undefined, { code: 'already_ruled', message: 'Already ruled by Bo' })));
    expect(within(cards()[0]).getByText('Already ruled by Bo')).toBeTruthy();
  });
});

describe('ruling from the keyboard', () => {
  it('j moves to the second ask, a opens a confirm row, Enter sends the rule, and "ruled" shows only after message.updated', () => {
    const m = mount([ask(1), ask(2)]);
    press('j');
    press('a');
    const row = screen.getByRole('group', { name: 'Confirm ruling' });
    expect(row.textContent).toContain('Option b');
    expect(rules(m.net)).toHaveLength(0);
    press('Enter');
    expect(rules(m.net).map((f) => f.data)).toEqual([{ decisionId: 'dec_2', optionId: 'b' }]);
    expect(screen.queryByText(/Ruled:/)).toBeNull();
    expect(within(cards()[1]).getAllByRole('button', { name: /Option/ })).toHaveLength(3);
    m.rulingOf(ask(2), 'b');
    const ruled = within(cards()[1]);
    expect(ruled.getByText(/Ruled: Option b by Ada/)).toBeTruthy();
    expect(ruled.queryByRole('button', { name: /Option/ })).toBeNull();
    expect(ruled.queryByText('blocked')).toBeNull();
    expect(ruled.getByText('ruled')).toBeTruthy();
    const when = ruled.getByText(/Nov 8/);
    expect(when.tagName).toBe('TIME');
    expect(when.getAttribute('datetime')).toBe('2026-11-08T11:00:00Z');
    expect(when.textContent).not.toMatch(/\d{4}-\d{2}-\d{2}T/);
  });

  it('1..n picks that option and opens the confirm row without ruling', () => {
    const m = mount([ask(1)]);
    press('1');
    expect(screen.getByRole('group', { name: 'Confirm ruling' }).textContent).toContain('Option a');
    expect(rules(m.net)).toHaveLength(0);
  });

  it('announces the confirm row through a live region', () => {
    mount([ask(1)]);
    const live = screen.getByRole('status');
    expect(live.textContent).toBe('');
    press('2');
    expect(live.textContent).toContain('Option b');
    expect(live.textContent).toContain('Enter');
  });

  it('Esc cancels the confirm row and sends nothing', () => {
    const m = mount([ask(1)]);
    press('a');
    press('Escape');
    expect(screen.queryByRole('group', { name: 'Confirm ruling' })).toBeNull();
    press('Enter');
    expect(rules(m.net)).toHaveLength(0);
  });

});

describe('ruling by mouse and moving focus', () => {
  it('rules by mouse through the same confirm row, and Cancel closes it', () => {
    const m = mount([ask(1)]);
    fireEvent.click(screen.getByRole('button', { name: /Option c/ }));
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByRole('group', { name: 'Confirm ruling' })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: /Option c/ }));
    fireEvent.click(screen.getByRole('button', { name: 'Rule' }));
    expect(rules(m.net).map((f) => f.data)).toEqual([{ decisionId: 'dec_1', optionId: 'c' }]);
  });

  it('moves focus to the next open ask after a ruling', () => {
    const m = mount([ask(1), ask(2)]);
    press('a');
    press('Enter');
    m.rulingOf(ask(1), 'b');
    expect(document.activeElement).toBe(cards()[1]);
  });

  it('gives the cards a visible, ordered focus target', () => {
    mount([ask(1), ask(2)]);
    cards().forEach((c) => expect(c.getAttribute('tabindex')).toBe('0'));
    press('j');
    expect(document.activeElement).toBe(cards()[1]);
    press('k');
    expect(document.activeElement).toBe(cards()[0]);
  });
});

describe('the composer', () => {
  it('typing a in the composer rules nothing, even with Enter', () => {
    const m = mount([ask(1)]);
    const box = composer();
    box.focus();
    press('a', box);
    expect(screen.queryByRole('group', { name: 'Confirm ruling' })).toBeNull();
    fireEvent.change(box, { target: { value: 'a' } });
    press('Enter', box);
    expect(rules(m.net)).toHaveLength(0);
    expect(m.net.sent().filter((f) => f.type === 'message.post').map((f) => f.data)).toEqual([{ text: 'a' }]);
  });

  it('posts message.post on Enter, clears the box, and refuses empty text', () => {
    const m = mount();
    press('Enter', composer());
    expect(m.net.sent().filter((f) => f.type === 'message.post')).toHaveLength(0);
    fireEvent.change(composer(), { target: { value: 'hello there' } });
    press('Enter', composer());
    expect(m.net.sent().filter((f) => f.type === 'message.post').map((f) => f.data)).toEqual([{ text: 'hello there' }]);
    expect((composer() as HTMLTextAreaElement).value).toBe('');
  });

  it('Shift+Enter is a newline and posts nothing', () => {
    const m = mount();
    fireEvent.change(composer(), { target: { value: 'line one' } });
    const notPrevented = press('Enter', composer(), { shiftKey: true });
    expect(notPrevented).toBe(true);
    expect(m.net.sent().filter((f) => f.type === 'message.post')).toHaveLength(0);
  });

  it('/ focuses the composer and types no slash', () => {
    mount([ask(1)]);
    const notPrevented = press('/');
    expect(notPrevented).toBe(false);
    expect(document.activeElement).toBe(composer());
  });
});

describe('the needs-you filter', () => {
  it('shows only the asks addressed to you and keeps a live count', () => {
    const m = mount([ask(1), theirs(2), note(3)]);
    expect(toggle().textContent).toBe('Needs you · 1');
    fireEvent.click(toggle());
    expect(toggle().getAttribute('aria-pressed')).toBe('true');
    expect(cards().map((c) => within(c).getByRole('heading', { level: 3 }).textContent)).toEqual(['Ask 1']);
    m.push(ask(4));
    expect(toggle().textContent).toBe('Needs you · 2');
    expect(cards()).toHaveLength(2);
  });

  it('drops the count to 0 after you rule, and says "Nothing needs you"', () => {
    const m = mount([ask(1), theirs(2)], '/pod/pod_1?filter=needs-you');
    press('a');
    press('Enter');
    m.rulingOf(ask(1), 'b');
    expect(toggle().textContent).toBe('Needs you · 0');
    expect(screen.getByText('Nothing needs you')).toBeTruthy();
    expect(screen.queryAllByRole('article')).toHaveLength(0);
  });

  it('counts an ask addressed to your role in the pod', () => {
    mount([ask(1, { needs: { to: { role: 'owner' }, action: 'approve' } }), ask(2, { needs: { to: { role: 'member' }, action: 'approve' } })]);
    expect(toggle().textContent).toBe('Needs you · 1');
    fireEvent.click(toggle());
    expect(cards().map((c) => within(c).getByRole('heading', { level: 3 }).textContent)).toEqual(['Ask 1']);
  });

  it('starts filtered from ?filter=needs-you', () => {
    mount([ask(1), theirs(2)], '/pod/pod_1?filter=needs-you');
    expect(toggle().getAttribute('aria-pressed')).toBe('true');
    expect(cards()).toHaveLength(1);
  });

  it('keeps the filter in the address, so a remount still shows it', () => {
    const m = mount([ask(1), theirs(2)]);
    fireEvent.click(toggle());
    m.remount();
    expect(toggle().getAttribute('aria-pressed')).toBe('true');
    expect(cards().map((c) => within(c).getByRole('heading', { level: 3 }).textContent)).toEqual(['Ask 1']);
  });

  it('g n toggles the filter on and off', () => {
    mount([ask(1), theirs(2)]);
    expect(cards()).toHaveLength(2);
    press('g');
    press('n');
    expect(toggle().getAttribute('aria-pressed')).toBe('true');
    expect(cards()).toHaveLength(1);
    press('g');
    press('n');
    expect(cards()).toHaveLength(2);
  });

  it('says "Nothing needs you" when the filter is on and no ask is yours', () => {
    mount([theirs(1)], '/pod/pod_1?filter=needs-you');
    expect(screen.getByText('Nothing needs you')).toBeTruthy();
  });
});

describe('who is working', () => {
  const scout = { kind: 'agent' as const, id: 'agt_1', displayName: 'Scout' };
  it('says an agent is working while its presence is working, and stops saying it when it goes back online', () => {
    const net = fakeNet();
    render(<MemoryRouter initialEntries={['/pod/pod_1']}><App connect={() => net.connect} http={fakeHttp().http} signedIn /></MemoryRouter>);
    act(() => {
      net.open();
      net.deliver(frame('hello', undefined, hello({ agents: [{ agent: scout, harness: 'fake', computerId: 'cmp_1', state: 'online' }] })));
    });
    expect(screen.queryByText('Scout is working.')).toBeNull();
    act(() => net.deliver(frame('presence.changed', undefined, { actor: scout, state: 'working', computerId: 'cmp_1' })));
    expect(screen.getByText('Scout is working.')).toBeTruthy();
    act(() => net.deliver(frame('presence.changed', undefined, { actor: scout, state: 'online', computerId: 'cmp_1' })));
    expect(screen.queryByText('Scout is working.')).toBeNull();
  });
});
