// Paper row: "sign-in and /device approval". The daemon's side is the real device routes; the page is the real client.
// GitHub itself is not reachable from a spec, so "coming back from GitHub" is the session cookie it would have set.
import { api, person } from './pod';
import { expect, signIn, test } from './fixtures';

interface Start { deviceCode: string; userCode: string; verificationUri: string }

/** One poll of the daemon's token route: the status and the body. */
async function poll(deviceCode: string): Promise<{ status: number; body: { code?: string; token?: string; user?: { id: string } } }> {
  const res = await fetch(`${process.env.JM_WEB_E2E_BASE}/v1/auth/device/token`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ deviceCode }) });
  return { status: res.status, body: (await res.json()) as { code?: string; token?: string; user?: { id: string } } };
}

test('a daemon\'s device code is approved at /device after sign-in, and the daemon receives its token', async ({ page }) => {
  const start = await api<Start>('POST', '/v1/auth/device', null);
  const path = `/device?code=${start.userCode}`;
  expect(new URL(start.verificationUri).pathname + new URL(start.verificationUri).search).toBe(path);

  // Signed out: the page shows the code and sends the person through GitHub, to come back here with the code kept.
  await page.goto(path);
  await expect(page.getByText(start.userCode)).toBeVisible();
  await expect(page.getByRole('link', { name: 'Sign in with GitHub to continue' })).toHaveAttribute('href', `/v1/auth/github/start?next=${encodeURIComponent(path)}`);
  await expect(page.getByRole('button', { name: 'Approve' })).toHaveCount(0);

  // Back from GitHub, signed in: the page asks, and loading it approved nothing.
  const ada = await person('Ada');
  await signIn(page, ada);
  await page.goto(path);
  await expect(page.getByLabel('Device code')).toHaveValue(start.userCode);
  expect(await poll(start.deviceCode)).toMatchObject({ status: 400, body: { code: 'authorization_pending' } });

  await page.getByRole('button', { name: 'Approve' }).click();
  await expect(page.getByText('Your terminal is signed in. You can close this page.')).toBeVisible();

  // The daemon polls (not faster than the server's interval) and receives a token for the person who approved.
  let got = await poll(start.deviceCode);
  for (let i = 0; got.body.code === 'slow_down' && i < 10; i++) {
    await new Promise((r) => setTimeout(r, 500));
    got = await poll(start.deviceCode);
  }
  expect(got.status).toBe(200);
  expect(got.body.user?.id).toBe(ada.user.id);
  expect(got.body.token).toMatch(/^jmu_/);
  expect(await api<{ id: string }>('GET', '/v1/me', got.body.token ?? '')).toMatchObject({ id: ada.user.id });
  expect((await poll(start.deviceCode)).body.code).toBe('invalid_grant'); // exchanged once
});

test('a code that is not waiting says so and leaves the page usable', async ({ page }) => {
  await signIn(page, await person('Ada'));
  await page.goto('/device?code=BCDF-GHJK');
  await page.getByRole('button', { name: 'Approve' }).click();
  await expect(page.getByRole('alert')).toHaveText('No device is waiting for that code. Check it against your terminal.');
  await expect(page.getByRole('button', { name: 'Approve' })).toBeEnabled();
});

test('a typo outside the code alphabet is called a typo, not an expired code', async ({ page }) => {
  await signIn(page, await person('Ada'));
  await page.goto('/device?code=ABCD-EFGH'); // A and E are not in the alphabet
  await page.getByRole('button', { name: 'Approve' }).click();
  await expect(page.getByRole('alert')).toHaveText("That isn't a valid code; check it against your terminal.");
  await expect(page.getByRole('button', { name: 'Approve' })).toBeEnabled();
});
