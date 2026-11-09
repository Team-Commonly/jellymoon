import { cleanup, fireEvent, render, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it } from 'vitest';
import { App } from './App';
import { ApiError } from './http';
import { fakeHttp, fakeNet } from './testkit';

afterEach(cleanup);
const open = (http: ReturnType<typeof fakeHttp>['http'], signedIn: boolean) =>
  render(<MemoryRouter initialEntries={['/']}><App connect={() => fakeNet().connect} http={http} signedIn={signedIn} /></MemoryRouter>);
const made = (name: string) => ({ id: 'pod_new', name, createdAt: '2026-11-08T10:00:00Z' });

describe('/', () => {
  it('signed out: says what Jellymoon is and links to sign-in coming back here, with no form', () => {
    const { http, created } = fakeHttp();
    const { getByRole, queryByRole } = open(http, false);
    expect(getByRole('link', { name: 'Sign in with GitHub to start a pod' }).getAttribute('href')).toBe('/v1/auth/github/start?next=%2F');
    expect(queryByRole('button', { name: 'Create' })).toBeNull();
    expect(queryByRole('textbox')).toBeNull();
    expect(created).toEqual([]);
  });

  it('signed in: creates the pod with the name and goes to it', async () => {
    const { http, created } = fakeHttp();
    const { getByRole, getByLabelText } = open(http, true);
    fireEvent.change(getByLabelText('Pod name'), { target: { value: 'Launch room' } });
    fireEvent.click(getByRole('button', { name: 'Create' }));
    await waitFor(() => expect(getByRole('banner').textContent).toContain('Launch room'));
    expect(created).toEqual(['Launch room']);
  });

  it('an empty name keeps Create disabled, and the field stops at the schema limit', () => {
    const { http, created } = fakeHttp();
    const { getByRole, getByLabelText } = open(http, true);
    const create = getByRole('button', { name: 'Create' }) as HTMLButtonElement;
    expect(create.disabled).toBe(true);
    expect(getByLabelText('Pod name').getAttribute('maxlength')).toBe('80');
    fireEvent.change(getByLabelText('Pod name'), { target: { value: '   ' } });
    expect(create.disabled).toBe(true);
    fireEvent.click(create);
    expect(created).toEqual([]);
    fireEvent.change(getByLabelText('Pod name'), { target: { value: 'A' } });
    expect(create.disabled).toBe(false);
  });

});

describe('/ when creating goes wrong or twice', () => {
  it('a failure shows the server message inline, keeps the name and stays put', async () => {
    const { http } = fakeHttp(undefined, undefined, () => Promise.reject(new ApiError(400, 'That name is taken.')));
    const { getByRole, getByLabelText, findByText } = open(http, true);
    fireEvent.change(getByLabelText('Pod name'), { target: { value: 'Launch room' } });
    fireEvent.click(getByRole('button', { name: 'Create' }));
    expect(await findByText('Could not create the pod: That name is taken.')).toBeTruthy();
    expect((getByLabelText('Pod name') as HTMLInputElement).value).toBe('Launch room');
    expect(getByRole('banner').textContent).not.toContain('pod_new');
    expect((getByRole('button', { name: 'Create' }) as HTMLButtonElement).disabled).toBe(false);
  });

  it('a network failure also says so', async () => {
    const { http } = fakeHttp(undefined, undefined, () => Promise.reject(new Error('offline')));
    const { getByRole, getByLabelText, findByText } = open(http, true);
    fireEvent.change(getByLabelText('Pod name'), { target: { value: 'x' } });
    fireEvent.click(getByRole('button', { name: 'Create' }));
    expect(await findByText('Could not create the pod: the server could not be reached. Try again.')).toBeTruthy();
  });

  it('a double submit is one POST', async () => {
    let release!: () => void;
    const { http, created } = fakeHttp(undefined, undefined, (name) => new Promise((r) => { release = () => r(made(name)); }));
    const { getByRole, getByLabelText } = open(http, true);
    fireEvent.change(getByLabelText('Pod name'), { target: { value: 'Launch room' } });
    const create = getByRole('button', { name: 'Create' });
    fireEvent.click(create);
    fireEvent.click(create);
    expect((create as HTMLButtonElement).disabled).toBe(true);
    release();
    await waitFor(() => expect(getByRole('banner').textContent).toContain('Launch room'));
    expect(created).toHaveLength(1);
  });
});
