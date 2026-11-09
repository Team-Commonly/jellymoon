// Paper row: "invite link join". A teammate with no install opens the link, signs in and joins; no daemon, no CLI.
// GitHub itself is not reachable from a spec, so "coming back from GitHub" is the session cookie it would have set.
import { api, fakeAgent, person } from './pod';
import { expect, signIn, test } from './fixtures';

test('a teammate with no install opens /invite/:token, signs in and joins the pod', async ({ page }) => {
  const ada = await person('Ada');
  const cy = await person('Cy');
  const { id } = await api<{ id: string }>('POST', '/v1/pods', ada.token, { name: 'Launch room' });
  const { code } = await api<{ code: string }>('POST', `/v1/pods/${id}/invites`, ada.token, { maxUses: 1 });
  const agent = await fakeAgent(id, ada, 'Scout');
  try {
    await agent.post({ summary: 'Started on the auth fix', kind: 'fyi', status: 'in_progress' });
    const path = `/invite/${code}`;

    // Signed out: the page offers GitHub, to come back to this invite. It does not join.
    await page.goto(path);
    await expect(page.getByRole('link', { name: 'Sign in with GitHub to join' })).toHaveAttribute('href', `/v1/auth/github/start?next=${encodeURIComponent(path)}`);
    await expect(page.getByRole('button', { name: 'Join' })).toHaveCount(0);

    // Back from GitHub, signed in: loading the page spends nothing; Join does.
    await signIn(page, cy);
    await page.goto(path);
    await page.getByRole('button', { name: 'Join' }).click();
    await expect(page).toHaveURL(new RegExp(`/pod/${id}$`));

    // The pod is Cy's now: the page's own socket (the cookie) got the pod's history.
    await expect(page.getByRole('heading', { name: 'Started on the auth fix' })).toBeVisible();

    // The one-use invite is spent: for the next person the link says so instead of joining them.
    await signIn(page, await person('Di'));
    await page.goto(path);
    await page.getByRole('button', { name: 'Join' }).click();
    await expect(page.getByRole('alert')).toHaveText('This invite has been used or has expired. Ask whoever sent it for a new one.');
  } finally {
    agent.close();
  }
});
