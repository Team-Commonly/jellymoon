import { act, cleanup, fireEvent, render } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it } from 'vitest';
import type { Actor } from '../../gen/ts/protocol';
import { App } from './App';
import { ApiError } from './http';
import { fakeHttp, fakeNet, frame, hello, you } from './testkit';

afterEach(cleanup);
const at = (path: string, me: Actor | null = null, http = fakeHttp().http) => render(<MemoryRouter initialEntries={[path]}><App connect={() => fakeNet().connect} http={http} me={me} /></MemoryRouter>);

describe('shell', () => {
  it('renders the top bar with the pod id and a sign-in button on /pod/:id', () => {
    const { getByText, getByRole } = at('/pod/pod_7');
    expect(getByRole('banner').textContent).toContain('pod_7');
    expect(getByText('Sign in').closest('a')).toBeTruthy();
  });

  it('signed out, the bar links to the real sign-in and comes back to this page, and names nobody', () => {
    const { getByRole, queryByRole, queryByText } = at('/pod/pod_7?filter=needs-you');
    expect(getByRole('link', { name: 'Sign in' }).getAttribute('href')).toBe('/v1/auth/github/start?next=%2Fpod%2Fpod_7%3Ffilter%3Dneeds-you');
    expect(queryByRole('button', { name: 'Sign out' })).toBeNull();
    expect(getByRole('banner').querySelector('[title]')).toBeNull();
    expect(queryByText('Signed in')).toBeNull();
    expect(queryByText('Not signed in')).toBeNull();
  });

  it('names the pod from hello, and shows its id only until hello arrives', () => {
    const net = fakeNet();
    const { getByRole } = render(<MemoryRouter initialEntries={['/pod/pod_1']}><App connect={() => net.connect} http={fakeHttp().http} me={you} /></MemoryRouter>);
    expect(getByRole('banner').textContent).toContain('Pod pod_1');
    act(() => { net.open(); net.deliver(frame('hello', undefined, hello({ podName: 'Launch room' }))); });
    expect(getByRole('banner').textContent).toContain('Launch room');
    expect(getByRole('banner').textContent).not.toContain('pod_1');
  });

  it('renders a not-found line for an unknown path', () => {
    const { getByText } = at('/nowhere');
    expect(getByText('Nothing here.')).toBeTruthy();
  });
});

describe('who is signed in, in the bar', () => {
  it('signed in, the bar names the person, offers Sign out and no sign-in', () => {
    const { getByRole, queryByRole } = at('/device', you);
    const name = getByRole('banner').querySelector('[title]');
    expect(name?.textContent).toBe('Ada');
    expect(name?.getAttribute('title')).toBe('Ada');
    expect(getByRole('button', { name: 'Sign out' })).toBeTruthy();
    expect(queryByRole('link', { name: 'Sign in' })).toBeNull();
  });

  it('falls back to the handle (the id) when the display name is blank', () => {
    const { getByRole } = at('/device', { ...you, displayName: '   ' });
    expect(getByRole('banner').querySelector('[title]')?.textContent).toBe('usr_you');
  });

  it('Sign out posts the logout once, then shows the signed-out page at /', async () => {
    let release = () => {};
    const h = fakeHttp(undefined, undefined, undefined, undefined, undefined, () => new Promise<void>((r) => { release = r; }));
    const { getByRole, findByRole, queryByRole } = at('/pod/pod_7', you, h.http);
    const out = getByRole('button', { name: 'Sign out' });
    fireEvent.click(out);
    fireEvent.click(out);
    expect(h.loggedOut.count).toBe(1);
    expect((out as HTMLButtonElement).disabled).toBe(true);
    await act(async () => release());
    expect(await findByRole('link', { name: 'Sign in with GitHub to start a pod' })).toBeTruthy();
    expect(getByRole('link', { name: 'Sign in' }).getAttribute('href')).toBe('/v1/auth/github/start?next=%2F');
    expect(queryByRole('button', { name: 'Sign out' })).toBeNull();
    expect(getByRole('banner').querySelector('[title]')).toBeNull();
    expect(h.loggedOut.count).toBe(1);
  });

  it('a failed sign-out says so and stays signed in', async () => {
    const h = fakeHttp(undefined, undefined, undefined, undefined, undefined, () => Promise.reject(new ApiError(500, 'request failed')));
    const { getByRole, findByRole } = at('/pod/pod_7', you, h.http);
    fireEvent.click(getByRole('button', { name: 'Sign out' }));
    expect((await findByRole('alert')).textContent).toBe('Could not sign out. Try again.');
    expect(getByRole('banner').querySelector('[title]')?.textContent).toBe('Ada');
    expect((getByRole('button', { name: 'Sign out' }) as HTMLButtonElement).disabled).toBe(false);
    expect(getByRole('banner').textContent).toContain('pod_7');
  });
});
