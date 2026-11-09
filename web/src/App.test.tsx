import { cleanup, render } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it } from 'vitest';
import { App } from './App';
import { rememberPod } from './podNames';
import { fakeHttp, fakeNet } from './testkit';

afterEach(cleanup);
const at = (path: string, signedIn = false) => render(<MemoryRouter initialEntries={[path]}><App connect={() => fakeNet().connect} http={fakeHttp().http} signedIn={signedIn} /></MemoryRouter>);

describe('shell', () => {
  it('renders the top bar with the pod id and a sign-in button on /pod/:id', () => {
    const { getByText, getByRole } = at('/pod/pod_7');
    expect(getByRole('banner').textContent).toContain('pod_7');
    expect(getByText('Sign in').closest('a')).toBeTruthy();
  });

  it('names the pod in the bar when the client knows it, and falls back to the id only until then', () => {
    rememberPod({ id: 'pod_8', name: 'Walk pod', createdAt: '2026-11-08T10:00:00Z' });
    const named = at('/pod/pod_8');
    expect(named.getByRole('banner').textContent).toContain('Walk pod');
    expect(named.getByRole('banner').textContent).not.toContain('pod_8');
  });

  it('signed out, the bar links to the real sign-in and comes back to this page', () => {
    const { getByRole, queryByText } = at('/pod/pod_7?filter=needs-you');
    expect(getByRole('link', { name: 'Sign in' }).getAttribute('href')).toBe('/v1/auth/github/start?next=%2Fpod%2Fpod_7%3Ffilter%3Dneeds-you');
    expect(queryByText('Signed in')).toBeNull();
    expect(queryByText('Not signed in')).toBeNull();
  });

  it('signed in, the bar says so and offers no sign-in', () => {
    const { getByText, queryByRole, queryByText } = at('/device', true);
    expect(getByText('Signed in')).toBeTruthy();
    expect(queryByRole('link', { name: 'Sign in' })).toBeNull();
    expect(queryByText('Sign in')).toBeNull();
  });

  it('renders a not-found line for an unknown path', () => {
    const { getByText } = at('/nowhere');
    expect(getByText('Nothing here.')).toBeTruthy();
  });
});
