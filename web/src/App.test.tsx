import { act, cleanup, render } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it } from 'vitest';
import { App } from './App';
import { fakeHttp, fakeNet, frame, hello } from './testkit';

afterEach(cleanup);
const at = (path: string, signedIn = false) => render(<MemoryRouter initialEntries={[path]}><App connect={() => fakeNet().connect} http={fakeHttp().http} signedIn={signedIn} /></MemoryRouter>);

describe('shell', () => {
  it('renders the top bar with the pod id and a sign-in button on /pod/:id', () => {
    const { getByText, getByRole } = at('/pod/pod_7');
    expect(getByRole('banner').textContent).toContain('pod_7');
    expect(getByText('Sign in').closest('a')).toBeTruthy();
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

  it('names the pod from hello, and shows its id only until hello arrives', () => {
    const net = fakeNet();
    const { getByRole } = render(<MemoryRouter initialEntries={['/pod/pod_1']}><App connect={() => net.connect} http={fakeHttp().http} signedIn /></MemoryRouter>);
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
