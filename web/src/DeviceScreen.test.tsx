import { cleanup, fireEvent, render, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it } from 'vitest';
import { App } from './App';
import { ApiError } from './http';
import { fakeHttp, fakeNet, you } from './testkit';

afterEach(cleanup);
const open = (http: ReturnType<typeof fakeHttp>['http'], signedIn: boolean, path = '/device?code=WDJB-MJHT') =>
  render(<MemoryRouter initialEntries={[path]}><App connect={() => fakeNet().connect} http={http} me={signedIn ? you : null} /></MemoryRouter>);
const reject = (status: number, code?: string) => () => Promise.reject(new ApiError(status, 'x', code));
const INVALID = "That isn't a valid code; check it against your terminal.";
const EXPIRED = 'This code expired or is not a device code. Start the sign-in again in your terminal.';
const LINK = 'Sign in with GitHub to continue';
const USED = 'This code was already used.';
const UNKNOWN = 'No device is waiting for that code. Check it against your terminal.';
const DENY = "This wasn't me";
const REFUSED = 'The code WDJB-MJHT will not sign anyone in, and the terminal that asked for it will stop waiting. You can close this page.';
const TOO_MANY = 'Too many tries. Wait a few minutes, then try again.';

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

describe('/device, a code typed by hand', () => {
  it.each(['ABCD-EFGH','WDJB-MJH', 'WDJB-MJHTT', 'WDJBMJHT-', 'WDJB MJHT', '1234-5678'])('%s is not the shape of a code: says so and sends nothing', async (typed) => {
    const { http, approved } = fakeHttp();
    const { getByRole, getByLabelText, findByText } = open(http, true, '/device');
    fireEvent.change(getByLabelText('Device code'), { target: { value: typed } });
    fireEvent.click(getByRole('button', { name: 'Approve' }));
    expect(await findByText(INVALID)).toBeTruthy();
    expect(approved).toEqual([]);
    expect((getByRole('button', { name: 'Approve' }) as HTMLButtonElement).disabled).toBe(false);
  });

  it.each(['wdjbmjht', 'WDJB-mjht'])('%s, lowercase or no hyphen, is sent as typed: the server normalizes it', async (typed) => {
    const { http, approved } = fakeHttp();
    const { getByRole, getByLabelText, findByText } = open(http, true, '/device');
    fireEvent.change(getByLabelText('Device code'), { target: { value: typed } });
    fireEvent.click(getByRole('button', { name: 'Approve' }));
    await findByText('Device approved');
    expect(approved).toEqual([typed]);
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
    [410, USED],
    [400, 'This code expired or is not a device code. Start the sign-in again in your terminal.'],
    [401, 'Your session ended.'],
    [403, 'The server refused this approval. Open the link your terminal printed, on the address it shows.'],
    [429, TOO_MANY],
    [500, 'Could not approve: the server returned an error (500). Try again.'],
  ])('%i says what failed and leaves Approve available', async (status, line) => {
    const { http } = fakeHttp(undefined, reject(status));
    const { getByRole, findByText } = open(http, true);
    fireEvent.click(getByRole('button', { name: 'Approve' }));
    expect(await findByText(line)).toBeTruthy();
    await waitFor(() => expect((getByRole('button', { name: 'Approve' }) as HTMLButtonElement).disabled).toBe(false));
  });

  it.each([
    ['invalid_input', INVALID],
    ['expired_token', EXPIRED],
  ])('a 400 with code %s is told apart by its code', async (code, line) => {
    const { http } = fakeHttp(undefined, reject(400, code));
    const { getByRole, findByText } = open(http, true);
    fireEvent.click(getByRole('button', { name: 'Approve' }));
    expect(await findByText(line)).toBeTruthy();
  });

  it('401 offers the sign-in link, back to /device with the code kept', async () => {
    const { http } = fakeHttp(undefined, reject(401));
    const { getByRole, findByRole } = open(http, true);
    fireEvent.click(getByRole('button', { name: 'Approve' }));
    expect((await findByRole('link', { name: 'Sign in again' })).getAttribute('href')).toBe('/v1/auth/github/start?next=%2Fdevice%3Fcode%3DWDJB-MJHT');
  });

  it('a network failure also says so', async () => {
    const { http } = fakeHttp(undefined, () => Promise.reject(new Error('offline')));
    const { getByRole, findByText } = open(http, true);
    fireEvent.click(getByRole('button', { name: 'Approve' }));
    expect(await findByText('Could not approve: the server could not be reached. Try again.')).toBeTruthy();
  });
});

describe("/device, pressing This wasn't me", () => {
  const denyWith = (deny?: (c: string) => Promise<void>) => fakeHttp(undefined, undefined, undefined, undefined, undefined, undefined, undefined, deny);

  it('sits next to Approve when signed in, and not signed out', () => {
    expect(open(fakeHttp().http, true).getByRole('button', { name: DENY })).toBeTruthy();
    cleanup();
    expect(open(fakeHttp().http, false).queryByRole('button', { name: DENY })).toBeNull();
  });

  it('refuses the code once, approves nothing, and says the terminal will stop; Approve is gone', async () => {
    const { http, approved, denied } = denyWith();
    const { getByRole, findByText, queryByRole } = open(http, true);
    const button = getByRole('button', { name: DENY });
    fireEvent.click(button);
    fireEvent.click(button);
    expect(await findByText('Code refused')).toBeTruthy();
    expect(await findByText(REFUSED)).toBeTruthy();
    expect(denied).toEqual(['WDJB-MJHT']);
    expect(approved).toEqual([]);
    expect(queryByRole('button', { name: 'Approve' })).toBeNull();
    expect(queryByRole('button', { name: DENY })).toBeNull();
  });

  it('a code not in the alphabet is called a typo and sends nothing', async () => {
    const { http, denied } = denyWith();
    const { getByRole, getByLabelText, findByText } = open(http, true, '/device');
    fireEvent.change(getByLabelText('Device code'), { target: { value: 'ABCD-EFGH' } });
    fireEvent.click(getByRole('button', { name: DENY }));
    expect(await findByText(INVALID)).toBeTruthy();
    expect(denied).toEqual([]);
  });

  it.each([[404, 'not_found'], [410, 'gone'], [400, 'expired_token']])('a refusal (%i %s) shows the unknown-code line and leaves both buttons', async (status, code) => {
    const { http } = denyWith(reject(status, code));
    const { getByRole, findByText } = open(http, true);
    fireEvent.click(getByRole('button', { name: DENY }));
    expect(await findByText(UNKNOWN)).toBeTruthy();
    await waitFor(() => expect((getByRole('button', { name: DENY }) as HTMLButtonElement).disabled).toBe(false));
    expect((getByRole('button', { name: 'Approve' }) as HTMLButtonElement).disabled).toBe(false);
  });

  it('429 says to wait, and leaves both buttons', async () => {
    const { http } = denyWith(reject(429, 'rate_limited'));
    const { getByRole, findByText } = open(http, true);
    fireEvent.click(getByRole('button', { name: DENY }));
    expect(await findByText(TOO_MANY)).toBeTruthy();
    await waitFor(() => expect((getByRole('button', { name: DENY }) as HTMLButtonElement).disabled).toBe(false));
    expect((getByRole('button', { name: 'Approve' }) as HTMLButtonElement).disabled).toBe(false);
  });

  it('401 says the session ended and offers the sign-in link', async () => {
    const { http } = denyWith(reject(401, 'unauthorized'));
    const { getByRole, findByRole } = open(http, true);
    fireEvent.click(getByRole('button', { name: DENY }));
    expect((await findByRole('link', { name: 'Sign in again' })).getAttribute('href')).toBe('/v1/auth/github/start?next=%2Fdevice%3Fcode%3DWDJB-MJHT');
  });
});
