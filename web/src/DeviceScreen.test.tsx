import { cleanup, fireEvent, render, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it } from 'vitest';
import { App } from './App';
import { ApiError } from './http';
import { fakeHttp, fakeNet } from './testkit';

afterEach(cleanup);
const open = (http: ReturnType<typeof fakeHttp>['http'], signedIn: boolean, path = '/device?code=WDJB-MJHT') =>
  render(<MemoryRouter initialEntries={[path]}><App connect={() => fakeNet().connect} http={http} signedIn={signedIn} /></MemoryRouter>);
const reject = (status: number) => () => Promise.reject(new ApiError(status, 'x'));
const LINK = 'Sign in with GitHub to continue';

describe('/device', () => {
  it('signed out: shows the code and links to sign-in, coming back to /device with the code kept; approves nothing', () => {
    const { http, approved } = fakeHttp();
    const { getByRole, getByText, queryByText } = open(http, false);
    expect(getByText('WDJB-MJHT')).toBeTruthy();
    expect(getByRole('link', { name: LINK }).getAttribute('href')).toBe('/v1/auth/github/start?next=%2Fdevice%3Fcode%3DWDJB-MJHT');
    expect(queryByText('Approve')).toBeNull();
    expect(approved).toEqual([]);
  });

  it('signed out with no code: the link comes back to /device', () => {
    const { getByRole } = open(fakeHttp().http, false, '/device');
    expect(getByRole('link', { name: LINK }).getAttribute('href')).toBe('/v1/auth/github/start?next=%2Fdevice');
  });

  it('signed in: loading approves nothing, and the code is in the box', async () => {
    const { http, approved } = fakeHttp();
    const { getByRole, getByLabelText } = open(http, true);
    await Promise.resolve();
    expect(approved).toEqual([]);
    expect((getByLabelText('Device code') as HTMLInputElement).value).toBe('WDJB-MJHT');
    expect(getByRole('button', { name: 'Approve' })).toBeTruthy();
  });

  it('Approve sends the code once, then says the terminal is signed in', async () => {
    const { http, approved } = fakeHttp();
    const { getByRole, findByText } = open(http, true);
    fireEvent.click(getByRole('button', { name: 'Approve' }));
    expect(await findByText('Your terminal is signed in. You can close this page.')).toBeTruthy();
    expect(approved).toEqual(['WDJB-MJHT']);
  });

  it('a typed code is what is sent, and an empty box sends nothing', async () => {
    const { http, approved } = fakeHttp();
    const { getByRole, getByLabelText, findByText } = open(http, true, '/device');
    fireEvent.click(getByRole('button', { name: 'Approve' }));
    await Promise.resolve();
    expect(approved).toEqual([]);
    fireEvent.change(getByLabelText('Device code'), { target: { value: ' wdjb-mjht ' } });
    fireEvent.click(getByRole('button', { name: 'Approve' }));
    await findByText('Device approved');
    expect(approved).toEqual(['wdjb-mjht']);
  });

});

describe('/device, pressing Approve', () => {
  it('a double click approves once', async () => {
    let release!: () => void;
    const { http, approved } = fakeHttp(undefined, () => new Promise((r) => { release = () => r(); }));
    const { getByRole, findByText } = open(http, true);
    const button = getByRole('button', { name: 'Approve' });
    fireEvent.click(button);
    fireEvent.click(button);
    release();
    await findByText('Device approved');
    expect(approved).toHaveLength(1);
  });

  it.each([
    [404, 'No device is waiting for that code. Check it against your terminal.'],
    [410, 'This code was already approved.'],
    [400, 'This code expired or is not a device code. Start the sign-in again in your terminal.'],
    [401, 'Your session ended. Sign in again.'],
    [500, 'Could not approve: the server returned an error (500). Try again.'],
  ])('%i says what failed and leaves Approve available', async (status, line) => {
    const { http } = fakeHttp(undefined, reject(status));
    const { getByRole, findByText } = open(http, true);
    fireEvent.click(getByRole('button', { name: 'Approve' }));
    expect(await findByText(line)).toBeTruthy();
    await waitFor(() => expect((getByRole('button', { name: 'Approve' }) as HTMLButtonElement).disabled).toBe(false));
  });

  it('a network failure also says so', async () => {
    const { http } = fakeHttp(undefined, () => Promise.reject(new Error('offline')));
    const { getByRole, findByText } = open(http, true);
    fireEvent.click(getByRole('button', { name: 'Approve' }));
    expect(await findByText('Could not approve: the server could not be reached. Try again.')).toBeTruthy();
  });
});
