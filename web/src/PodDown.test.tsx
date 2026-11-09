// /pod/:id when the socket is not there: a pod the server refuses for good, and a socket that dropped and comes back.
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { App } from './App';
import type { PodAccess } from './http';
import { fakeHttp, fakeNet, frame, hello } from './testkit';

afterEach(() => { cleanup(); vi.useRealTimers(); });

function open(path: string, access: PodAccess) {
  vi.useFakeTimers();
  const net = fakeNet();
  const h = fakeHttp(undefined, undefined, undefined, undefined, undefined, async () => access);
  render(<MemoryRouter initialEntries={[path]}><App connect={() => net.connect} http={h.http} signedIn /></MemoryRouter>);
  return { net, h };
}
const composer = () => screen.getByRole('textbox', { name: 'Message' }) as HTMLTextAreaElement;
const later = () => act(async () => void vi.advanceTimersByTime(60_000));

describe('a pod the server refuses', () => {
  // The fake socket closes before any hello, as a browser's does when the upgrade is answered 404 or 403.
  const refused = async (access: PodAccess) => {
    const r = open('/pod/pod_unknown', access);
    await act(async () => { r.net.drop(); });
    return r;
  };

  it('says there is no such pod or you are not a member, offers no composer or filter, and never retries the socket', async () => {
    const r = await refused('refused');
    expect(r.h.asked).toEqual(['pod_unknown']);
    expect(screen.getByRole('alert').textContent).toBe("No such pod, or you're not a member.");
    expect(screen.queryByRole('textbox', { name: 'Message' })).toBeNull();
    expect(screen.queryByRole('button', { name: /Needs you/ })).toBeNull();
    await later();
    expect(r.net.live).toHaveLength(1);
  });

  it('says to sign in when the server says nobody is signed in, and never retries the socket', async () => {
    const r = await refused('signed-out');
    expect(screen.getByRole('alert').textContent).toBe('Sign in to open this pod.');
    expect(screen.queryByRole('textbox', { name: 'Message' })).toBeNull();
    await later();
    expect(r.net.live).toHaveLength(1);
  });

  it('keeps retrying a member\'s pod whose socket closed before hello, and shows it once hello comes', async () => {
    const r = open('/pod/pod_1', 'member');
    await act(async () => { r.net.drop(); });
    expect(screen.queryByRole('alert')).toBeNull();
    await act(async () => void vi.advanceTimersByTime(1000));
    expect(r.net.live).toHaveLength(2);
    act(() => { r.net.open(); r.net.deliver(frame('hello', undefined, hello())); });
    expect(composer().disabled).toBe(false);
  });
});

describe('a dropped socket', () => {
  it('disables the composer and says it is reconnecting, sends nothing meanwhile, and comes back with the next hello', async () => {
    const { net } = open('/pod/pod_1', 'member');
    expect(composer().disabled).toBe(true); // no hello yet
    act(() => { net.open(); net.deliver(frame('hello', undefined, hello())); });
    fireEvent.change(composer(), { target: { value: 'hello?' } });
    act(() => net.drop());
    expect(composer().disabled).toBe(true);
    expect((screen.getByRole('button', { name: 'Send' }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText('Reconnecting…')).toBeTruthy();
    fireEvent.keyDown(composer(), { key: 'Enter' });
    fireEvent.submit(composer().closest('form')!);
    expect(net.live.flatMap((s) => s.sent).filter((t) => t.includes('message.post'))).toHaveLength(0);
    expect(composer().value).toBe('hello?');
    await act(async () => void vi.advanceTimersByTime(1000));
    act(() => net.open());
    expect(composer().disabled).toBe(true);
    act(() => net.deliver(frame('hello', undefined, hello())));
    expect(composer().disabled).toBe(false);
    expect(screen.queryByText('Reconnecting…')).toBeNull();
    fireEvent.keyDown(composer(), { key: 'Enter' });
    expect(net.sent().filter((f) => f.type === 'message.post').map((f) => f.data)).toEqual([{ text: 'hello?' }]);
  });
});
