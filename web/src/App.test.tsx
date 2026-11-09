import { cleanup, render } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it } from 'vitest';
import { App } from './App';
import { fakeHttp, fakeNet } from './testkit';

afterEach(cleanup);
const at = (path: string) => render(<MemoryRouter initialEntries={[path]}><App connect={() => fakeNet().connect} http={fakeHttp().http} signedIn={false} /></MemoryRouter>);

describe('shell', () => {
  it('renders the top bar with the pod id and a sign-in button on /pod/:id', () => {
    const { getByText, getByRole } = at('/pod/pod_7');
    expect(getByRole('banner').textContent).toContain('pod_7');
    expect(getByText('Sign in')).toBeTruthy();
  });

  it('renders a not-found line for an unknown path', () => {
    const { getByText } = at('/nowhere');
    expect(getByText('Nothing here.')).toBeTruthy();
  });
});
