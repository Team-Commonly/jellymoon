// The bar names who is signed in and offers Sign out, which is a real cookie POST to /v1/auth/logout. GitHub itself is not reachable from a spec, so "coming back from GitHub" is the session cookie it would have set.
import { api, person } from './pod';
import { expect, signIn, test } from './fixtures';

test('a signed-in person sees their name in the bar, signs out from a pod page, and lands on / signed out', async ({ page }) => {
  const ada = await person('Ada');
  const { id } = await api<{ id: string }>('POST', '/v1/pods', ada.token, { name: 'Launch room' });
  await signIn(page, ada);
  await page.goto(`/pod/${id}`);
  const bar = page.getByRole('banner');
  await expect(bar).toContainText('Launch room');
  await expect(bar.locator('[title="Ada"]')).toHaveText('Ada');

  await bar.getByRole('button', { name: 'Sign out' }).click();
  await expect(page).toHaveURL(/\/$/);
  await expect(bar.getByRole('link', { name: 'Sign in' })).toBeVisible();
  await expect(bar.getByRole('button', { name: 'Sign out' })).toHaveCount(0);
  await expect(page.getByRole('link', { name: 'Sign in with GitHub to start a pod' })).toBeVisible();

  // The server revoked the token the cookie held and cleared the cookie: a reload is still signed out.
  await expect(api('GET', '/v1/me', ada.token)).rejects.toThrow(/HTTP 401/);
  await page.reload();
  await expect(bar.getByRole('link', { name: 'Sign in' })).toBeVisible();
});
