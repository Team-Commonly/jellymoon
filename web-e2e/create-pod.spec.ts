// Paper row: "create a pod from the web". A new person signs in, opens /, names a pod and lands on it.
// GitHub itself is not reachable from a spec, so "coming back from GitHub" is the session cookie it would have set.
import { fakeAgent, person } from './pod';
import { expect, signIn, test } from './fixtures';

test('a signed-in person opens /, creates "Launch room" and lands on its pod page', async ({ page }) => {
  // Signed out: one line on what this is and the way in, no form.
  await page.goto('/');
  await expect(page.getByRole('link', { name: 'Sign in with GitHub to start a pod' })).toHaveAttribute('href', `/v1/auth/github/start?next=${encodeURIComponent('/')}`);
  await expect(page.getByRole('button', { name: 'Create' })).toHaveCount(0);

  // Back from GitHub: an empty name cannot be sent; a name can.
  const ada = await person('Ada');
  await signIn(page, ada);
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Create' })).toBeDisabled();
  await page.getByLabel('Pod name').fill('Launch room');
  await page.getByRole('button', { name: 'Create' }).click();
  await expect(page).toHaveURL(/\/pod\/[^/]+$/);
  const id = new URL(page.url()).pathname.split('/').pop() ?? '';

  // The pod is Ada's and the page loaded it: a message an agent posts there reaches the page's own socket (the cookie).
  const agent = await fakeAgent(id, ada, 'Scout');
  try {
    await agent.post({ summary: 'Started on the auth fix', kind: 'fyi', status: 'in_progress' });
    await expect(page.getByRole('heading', { name: 'Started on the auth fix' })).toBeVisible();
    await expect(page.getByRole('banner')).toContainText('Launch room');
    await expect(page.getByRole('banner')).not.toContainText(id);
  } finally {
    agent.close();
  }
});
