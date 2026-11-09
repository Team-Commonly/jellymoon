import { act, cleanup, fireEvent, render, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it } from 'vitest';
import { App } from './App';
import { ApiError } from './http';
import { fakeHttp, fakeNet, PREVIEW } from './testkit';

afterEach(cleanup);
const GONE = 'This invite has been used or has expired. Ask whoever sent it for a new one.';
const open = (http: ReturnType<typeof fakeHttp>['http'], signedIn: boolean, path = '/invite/abc123') =>
  render(<MemoryRouter initialEntries={[path]}><App connect={() => fakeNet().connect} http={http} signedIn={signedIn} /></MemoryRouter>);

describe('/invite/:token', () => {
  it('signed out: shows the sign-in line linking to sign-in with a return path, and no Join', () => {
    const { http, calls } = fakeHttp();
    const { getByRole, queryByText } = open(http, false);
    const link = getByRole('link', { name: 'Sign in with GitHub to join' });
    expect(link.getAttribute('href')).toBe('/v1/auth/github/start?next=%2Finvite%2Fabc123');
    expect(queryByText('Join')).toBeNull();
    expect(calls).toEqual([]);
  });

  it('signed in: loading the page calls accept zero times and offers Join', async () => {
    const { http, calls } = fakeHttp();
    const { getByRole } = open(http, true);
    await Promise.resolve();
    expect(calls).toEqual([]);
    expect(getByRole('button', { name: 'Join' })).toBeTruthy();
  });

  it('Join calls accept exactly once with the code, then goes to the pod', async () => {
    const { http, calls } = fakeHttp();
    const { getByRole } = open(http, true);
    fireEvent.click(getByRole('button', { name: 'Join' }));
    await waitFor(() => expect(getByRole('banner').textContent).toContain('pod_9'));
    expect(calls).toEqual(['abc123']);
  });

  it('a double click joins once', async () => {
    let release!: () => void;
    const { http, calls } = fakeHttp(() => new Promise((r) => { release = () => r({ pod: { id: 'pod_9', name: 'P', createdAt: '2026-11-08T10:00:00Z' } }); }));
    const { getByRole } = open(http, true);
    const join = getByRole('button', { name: 'Join' });
    fireEvent.click(join);
    fireEvent.click(join);
    release();
    await waitFor(() => expect(getByRole('banner').textContent).toContain('pod_9'));
    expect(calls).toHaveLength(1);
  });

  it('410 shows the gone line and stays on the page', async () => {
    const { http } = fakeHttp(() => Promise.reject(new ApiError(410, 'gone')));
    const { getByRole, findByText } = open(http, true);
    fireEvent.click(getByRole('button', { name: 'Join' }));
    expect(await findByText(GONE)).toBeTruthy();
  });

  it('another failure says what failed instead of a blank screen', async () => {
    const { http } = fakeHttp(() => Promise.reject(new ApiError(500, 'internal')));
    const { getByRole, findByText } = open(http, true);
    fireEvent.click(getByRole('button', { name: 'Join' }));
    expect(await findByText('Could not join: the server returned an error (500). Try again.')).toBeTruthy();
  });

  it('a network failure also says so', async () => {
    const { http } = fakeHttp(() => Promise.reject(new Error('offline')));
    const { getByRole, findByText } = open(http, true);
    fireEvent.click(getByRole('button', { name: 'Join' }));
    expect(await findByText('Could not join: the server could not be reached. Try again.')).toBeTruthy();
  });
});

describe('/invite/:token, the invite read', () => {
  it('names the pod and who sent the invite, signed out and signed in', async () => {
    for (const signedIn of [false, true]) {
      const { findByRole, unmount } = open(fakeHttp().http, signedIn);
      expect((await findByRole('heading', { name: `${PREVIEW.inviterName} invited you to ${PREVIEW.podName}` }))).toBeTruthy();
      unmount();
    }
  });

  it('when the invite read 404s the page keeps its plain line, and Join still joins', async () => {
    const { http, calls } = fakeHttp(undefined, undefined, undefined, undefined, async () => null);
    const { getByRole } = open(http, true);
    await act(async () => {}); // the read has answered
    expect(getByRole('heading').textContent).toBe('You have been invited to a pod');
    fireEvent.click(getByRole('button', { name: 'Join' }));
    await waitFor(() => expect(calls).toEqual(['abc123']));
  });

  it('a failed invite read is the plain line too', async () => {
    const { http } = fakeHttp(undefined, undefined, undefined, undefined, async () => { throw new Error('offline'); });
    const { getByRole } = open(http, false);
    await act(async () => {});
    expect(getByRole('heading').textContent).toBe('You have been invited to a pod');
  });
});
