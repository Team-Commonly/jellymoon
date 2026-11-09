import { cleanup, render } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it } from 'vitest';
import { App } from './App';

afterEach(cleanup);
const at = (path: string) => render(<MemoryRouter initialEntries={[path]}><App /></MemoryRouter>);

describe('shell', () => {
  it('renders the top bar and a placeholder for /pod/:id', () => {
    const { getByText, getByRole } = at('/pod/pod_7');
    expect(getByRole('banner').textContent).toContain('pod_7');
    expect(getByText('Sign in')).toBeTruthy();
    expect(getByText('Pod pod_7 will show here.')).toBeTruthy();
  });

  it('renders a not-found line for an unknown path', () => {
    const { getByText } = at('/nowhere');
    expect(getByText('Nothing here.')).toBeTruthy();
  });
});
