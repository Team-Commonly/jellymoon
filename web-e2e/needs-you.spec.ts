// The needs-you filter (the paper's third screen) on the same stream: only the open asks addressed to you, a live count,
// "Nothing needs you" once you have ruled, and ?filter=needs-you kept across a reload.
import { askInput, decisionOf } from './pod';
import { expect, openPod, test } from './fixtures';

const MINE = 'Ship the login fix tonight?';
const THEIRS = 'Rename the billing table?';

test('the filter shows exactly the asks addressed to you, counts them live, and stays on across a reload', async ({ page, pod }) => {
  const mine = await pod.agent.post(askInput(MINE, { userIds: [pod.you.user.id] }, pod.agent.cited));
  const theirs = await pod.agent.post(askInput(THEIRS, { userIds: [pod.other.user.id] }, pod.agent.cited));
  await openPod(page, pod);
  const toggle = page.getByRole('button', { name: /^Needs you · / });
  const heading = (name: string) => page.getByRole('heading', { name });

  // Unfiltered, both asks are in the stream; only one of them is yours.
  await expect(heading(MINE)).toBeVisible();
  await expect(heading(THEIRS)).toBeVisible();
  await expect(toggle).toHaveText('Needs you · 1');
  await expect(toggle).toHaveAttribute('aria-pressed', 'false');

  // g n turns the filter on: exactly your ask, and the URL says so.
  await page.keyboard.press('g');
  await page.keyboard.press('n');
  await expect(toggle).toHaveAttribute('aria-pressed', 'true');
  await expect(page).toHaveURL(/[?&]filter=needs-you(&|$)/);
  await expect(page.getByRole('article')).toHaveCount(1);
  await expect(heading(MINE)).toBeVisible();
  await expect(heading(THEIRS)).toHaveCount(0);

  // A reload of the filtered URL keeps the filter on. The other ask is replayed to the page and still filtered out.
  await openPod(page, pod, () => page.reload());
  await expect.poll(() => pod.bridge.sent('message.created', (d) => d.id === theirs.messageId), { message: 'the replay carried the other ask' }).toBe(true);
  await expect(page).toHaveURL(/[?&]filter=needs-you(&|$)/);
  await expect(toggle).toHaveAttribute('aria-pressed', 'true');
  await expect(heading(MINE)).toBeVisible();
  await expect(toggle).toHaveText('Needs you · 1');
  // Off and on again, so the filtering after the reload is shown on a page that has rendered both asks.
  await page.keyboard.press('g');
  await page.keyboard.press('n');
  await expect(toggle).toHaveAttribute('aria-pressed', 'false');
  await expect(heading(MINE)).toBeVisible();
  await expect(heading(THEIRS)).toBeVisible();
  await page.keyboard.press('g');
  await page.keyboard.press('n');
  await expect(toggle).toHaveAttribute('aria-pressed', 'true');
  await expect(page).toHaveURL(/[?&]filter=needs-you(&|$)/);
  await expect(page.getByRole('article')).toHaveCount(1);
  await expect(heading(MINE)).toBeVisible();
  await expect(heading(THEIRS)).toHaveCount(0);

  // Rule yours from the keyboard: the count drops to 0 and the filter says nothing needs you.
  await page.keyboard.press('a');
  await expect(page.getByRole('group', { name: 'Confirm ruling' })).toContainText('Rule Merge now?');
  await page.keyboard.press('Enter');
  await expect(toggle).toHaveText('Needs you · 0');
  await expect(page.getByText('Nothing needs you')).toBeVisible();
  await expect(page.getByRole('article')).toHaveCount(0);
  expect(await decisionOf(pod.id, pod.you.token, mine.decisionId ?? '')).toEqual({ state: 'ruled', optionId: 'a', by: pod.you.user.id });
  expect(await decisionOf(pod.id, pod.you.token, theirs.decisionId ?? '')).toEqual({ state: 'open' });

  // Off again, the other person's ask is still there and still open.
  await page.keyboard.press('g');
  await page.keyboard.press('n');
  await expect(toggle).toHaveAttribute('aria-pressed', 'false');
  await expect(page.getByRole('article').filter({ has: heading(THEIRS) }).getByRole('group', { name: 'Options' })).toBeVisible();
});
