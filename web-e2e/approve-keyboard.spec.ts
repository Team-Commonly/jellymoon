// Paper row: "Open pod, see card, press a, ruled state appears (WS live)". Keys only: no click, no tap, no mouse.
import { askInput, decisionOf } from './pod';
import { expect, openPod, test } from './fixtures';

const ASK = 'Merge the auth fix now or wait for review?';

test('an ask that arrives live is ruled from the keyboard, and the card shows the ruling only once the pod has sent it', async ({ page, pod }) => {
  const loads: string[] = [];
  page.on('framenavigated', (f) => { if (f === page.mainFrame()) loads.push(f.url()); });
  await pod.agent.post({ summary: 'Started on the auth fix', kind: 'fyi', status: 'in_progress' });
  await openPod(page, pod);
  await expect(page.getByRole('heading', { name: 'Started on the auth fix' })).toBeVisible();

  // The fake agent asks while the page is open: the card arrives over the socket, with no reload and no second socket.
  const asked = await pod.agent.post(askInput(ASK, { userIds: [pod.you.user.id] }));
  const decisionId = asked.decisionId ?? '';
  const card = page.getByRole('article').filter({ has: page.getByRole('heading', { name: ASK }) });
  await expect(card).toBeVisible();
  await expect(card.getByRole('group', { name: 'Options' })).toBeVisible();
  expect(loads).toHaveLength(1);
  expect(pod.bridge.connections).toBe(1);

  // In the composer, `a` and Enter are text: nothing is confirmed or ruled.
  const composer = page.getByRole('textbox', { name: 'Message' });
  await page.keyboard.press('/');
  await expect(composer).toBeFocused();
  await page.keyboard.press('a');
  await expect(composer).toHaveValue('a');
  await expect(page.getByRole('group', { name: 'Confirm ruling' })).toHaveCount(0);
  await expect(page.getByRole('status')).toHaveText('');
  await page.keyboard.press('Backspace');
  await page.keyboard.press('Enter'); // an empty composer sends nothing
  await page.keyboard.type('Looking at it now');
  await page.keyboard.press('Enter');
  await expect(page.getByRole('heading', { name: 'Looking at it now' })).toBeVisible();
  // One socket's frames are handled in order, so by the time that post is back the pod has seen anything typed before it.
  expect(pod.bridge.fromPage.filter((f) => f.type === 'decision.rule')).toEqual([]);
  expect(pod.bridge.fromPage.filter((f) => f.type === 'message.post').map((f) => f.data.text)).toEqual(['Looking at it now']);
  expect(await decisionOf(pod.id, pod.you.token, decisionId)).toEqual({ state: 'open' });
  await expect(card.getByRole('group', { name: 'Options' })).toBeVisible();

  // Out of the composer, j focuses the ask, a opens the confirm row on the recommended option, Enter rules.
  await page.keyboard.press('Tab');
  await expect(composer).not.toBeFocused();
  await page.keyboard.press('j');
  await expect(card).toBeFocused();
  await page.keyboard.press('a');
  await expect(card.getByRole('group', { name: 'Confirm ruling' })).toContainText('Rule Merge now?');
  pod.bridge.hold();
  await page.keyboard.press('Enter');
  await expect.poll(() => pod.bridge.fromPage.filter((f) => f.type === 'decision.rule').map((f) => f.data)).toEqual([{ decisionId, optionId: 'a' }]);
  await expect.poll(() => pod.bridge.sent('message.updated', (d) => d.id === asked.messageId), { message: 'the pod sent message.updated' }).toBe(true);

  // The pod has ruled and said so, but the page has not been told: it still shows the ask open and waiting.
  expect(await decisionOf(pod.id, pod.you.token, decisionId)).toEqual({ state: 'ruled', optionId: 'a', by: pod.you.user.id });
  await expect(card.getByText('Sending your ruling, waiting for the pod.')).toBeVisible();
  await expect(card.getByText(/^Ruled:/)).toHaveCount(0);
  await expect(card.getByRole('group', { name: 'Options' })).toBeVisible();

  // message.updated reaches the page: the ruling shows and the options go.
  pod.bridge.release();
  await expect(card.getByText('Ruled: Merge now by Ada')).toBeVisible();
  await expect(card.getByRole('group', { name: 'Options' })).toHaveCount(0);
  await expect(card.getByText('Sending your ruling, waiting for the pod.')).toHaveCount(0);
  expect(loads).toHaveLength(1);
});
