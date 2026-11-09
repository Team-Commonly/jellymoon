// Paper row: "layout invariants on each route", at 390 and 1440 px (the two projects). Two invariants, on every route and in the
// states a person reaches on it:
//   1. No horizontal page scroll: the document is no wider than the viewport.
//   2. No clipped text. Text is clipped when an element that has text of its own, or an ancestor of it, hides overflow
//      (overflow hidden or clip) and the text runs past that box, sideways or down. Scrolling boxes (auto, scroll) do not clip: a
//      person can reach what is in them. The one allowance is a line that deliberately ellipsizes (text-overflow: ellipsis) and
//      carries a title attribute with the whole text. Form controls are left out: a browser scrolls their value on its own.
//      It is measured on the text's own rectangle (a Range) with a 1px tolerance for subpixel rounding, so an element that is
//      not showing, or a box that merely has room to spare, never counts.
import type { Page } from '@playwright/test';
import { api, askInput, fakeAgent, person } from './pod';
import { expect, openPod, signIn, test } from './fixtures';

interface Finding { what: string; text: string; detail: string }

/** Runs in the page. Returns what is clipped; an empty list is the pass. */
async function clippedText(page: Page): Promise<Finding[]> {
  return page.evaluate(() => {
    const out: Array<{ what: string; text: string; detail: string }> = [];
    const clips = (v: string) => v === 'hidden' || v === 'clip';
    const name = (el: Element) => `${el.tagName.toLowerCase()}${el.getAttribute('data-chip') ? `[${el.getAttribute('data-chip')}]` : ''}`;
    for (const el of Array.from(document.body.querySelectorAll('*'))) {
      if (['SCRIPT', 'STYLE', 'INPUT', 'TEXTAREA', 'SELECT', 'OPTION'].includes(el.tagName)) continue;
      const own = Array.from(el.childNodes).filter((n) => n.nodeType === Node.TEXT_NODE && (n.textContent ?? '').trim() !== '');
      if (own.length === 0) continue;
      const style = getComputedStyle(el);
      if (style.visibility === 'hidden' || style.display === 'none') continue;
      const range = document.createRange();
      range.setStart(own[0], 0);
      range.setEnd(own[own.length - 1], (own[own.length - 1].textContent ?? '').length);
      const box = range.getBoundingClientRect();
      if (box.width === 0 && box.height === 0) continue;
      const text = (el.textContent ?? '').trim().slice(0, 60);
      const ellipsized = style.textOverflow === 'ellipsis' && (el.getAttribute('title') ?? '').trim() !== '';
      for (let a: Element | null = el; a && a !== document.documentElement; a = a.parentElement) {
        const s = getComputedStyle(a);
        const xClips = clips(s.overflowX);
        const yClips = clips(s.overflowY);
        if (!xClips && !yClips) continue;
        if (a === el && ellipsized) continue;
        const r = a.getBoundingClientRect();
        const sideways = xClips && (box.right > r.right + 1 || box.left < r.left - 1);
        const down = yClips && (box.bottom > r.bottom + 1 || box.top < r.top - 1);
        if (sideways || down) out.push({ what: name(el), text, detail: `${sideways ? 'sideways' : 'down'} past ${name(a)}` });
      }
      if (clips(style.overflowX) && !ellipsized && el.scrollWidth > el.clientWidth + 1) out.push({ what: name(el), text, detail: `scrollWidth ${el.scrollWidth} > clientWidth ${el.clientWidth}` });
    }
    return out;
  });
}

/** The two invariants, on the page as it stands. */
async function holdsLayout(page: Page, where: string): Promise<void> {
  await expect(page.getByRole('banner'), `${where}: renders inside the shell`).toBeVisible();
  const w = await page.evaluate(() => ({ scroll: document.documentElement.scrollWidth, client: document.documentElement.clientWidth, body: document.body.scrollWidth }));
  expect(w.scroll, `${where}: horizontal page scroll (document ${w.scroll}px in a ${w.client}px viewport)`).toBeLessThanOrEqual(w.client);
  expect(w.body, `${where}: the body is wider than the viewport (${w.body}px in ${w.client}px)`).toBeLessThanOrEqual(w.client);
  expect(await clippedText(page), `${where}: clipped text`).toEqual([]);
}

// A name no browser can break at a space: the worst case for a narrow screen.
const UNBROKEN = 'Refactor_the_session_middleware_and_every_downstream_consumer_of_the_token_refresh_path_before_friday';
// A summary is one line of at most 120 characters (schema/message.json); the server refuses a longer one with a 400.
const ASK_SUMMARY = `Ship ${UNBROKEN}?`;

test('the seeded summaries fit the schema\'s 120 characters', () => {
  expect(UNBROKEN.length).toBeLessThanOrEqual(120);
  expect(ASK_SUMMARY.length).toBeLessThanOrEqual(120);
});

test('the layout check sees what it should: clipped text and a wide page are caught, a titled ellipsis is not', async ({ page }) => {
  await page.goto('/device');
  await holdsLayout(page, 'control: the plain page');
  const long = 'Clipped words that run on and on '.repeat(6);

  await page.evaluate((text) => {
    const d = document.createElement('div');
    d.id = 'hidden-box';
    d.style.cssText = 'width:80px;overflow:hidden;white-space:nowrap';
    d.textContent = text;
    document.body.append(d);
  }, long);
  expect((await clippedText(page)).length).toBeGreaterThan(0);

  await page.evaluate((text) => {
    document.getElementById('hidden-box')?.remove();
    const p = document.createElement('p');
    p.title = text;
    p.style.cssText = 'width:80px;overflow:hidden;white-space:nowrap;text-overflow:ellipsis';
    p.textContent = text;
    document.body.append(p);
  }, long);
  expect(await clippedText(page)).toEqual([]);

  await page.evaluate(() => {
    const wide = document.createElement('div');
    wide.style.cssText = 'width:3000px;height:4px';
    document.body.append(wide);
  });
  await expect(holdsLayout(page, 'control: a wide page')).rejects.toThrow(/horizontal page scroll/);
});

test('the top bar keeps its layout under a pod id as long as an address can make it', async ({ page }) => {
  await page.goto(`/pod/${UNBROKEN}${UNBROKEN}`);
  await expect(page.getByRole('banner')).toContainText('Sign in');
  await holdsLayout(page, 'the bar with a very long pod id');
  await signIn(page, await person('Ada'));
  await page.goto(`/pod/${UNBROKEN}${UNBROKEN}`);
  await expect(page.getByRole('banner')).toContainText('Signed in');
  await holdsLayout(page, 'the bar with a very long pod id, signed in');
});

test('/device keeps its layout signed out, signed in, with an error showing, and approved', async ({ page }) => {
  const start = await api<{ userCode: string }>('POST', '/v1/auth/device', null);
  const path = `/device?code=${start.userCode}`;

  await page.goto(path);
  await expect(page.getByRole('link', { name: 'Sign in with GitHub to continue' })).toBeVisible();
  await holdsLayout(page, '/device signed out');

  await signIn(page, await person('Ada'));
  await page.goto(path);
  await expect(page.getByLabel('Device code')).toHaveValue(start.userCode);
  await holdsLayout(page, '/device signed in');

  // The error line, from a code that is not waiting.
  await page.getByLabel('Device code').fill('BCDF-GHJK');
  await page.getByRole('button', { name: 'Approve' }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  await holdsLayout(page, '/device with an error');

  await page.getByLabel('Device code').fill(start.userCode);
  await page.getByRole('button', { name: 'Approve' }).click();
  await expect(page.getByText('Your terminal is signed in. You can close this page.')).toBeVisible();
  await holdsLayout(page, '/device approved');
});

test('/pod/:id keeps its layout with cards: an unbroken headline, an open body, an ask, a confirm row and the composer', async ({ page, pod }) => {
  const body = `${UNBROKEN}\n${'A long line of body text that has to wrap and never push the page wider than the screen. '.repeat(4)}`;
  await pod.agent.post({ summary: UNBROKEN, kind: 'fyi', status: 'in_progress', body });
  await pod.agent.post({ summary: 'Tests green on the fix branch', kind: 'fyi', status: 'done' });
  await pod.agent.post({ ...askInput(ASK_SUMMARY, { userIds: [pod.you.user.id] }), body: 'The diff touches only the session middleware.' });
  await openPod(page, pod);
  await expect(page.getByRole('article')).toHaveCount(3);
  await holdsLayout(page, '/pod/:id folded');

  // The card's parts the paper names are all on screen: summary line, status chip, the pod-stamped receipt chip, and a folded body.
  const first = page.getByRole('article').filter({ has: page.getByRole('heading', { name: 'Tests green on the fix branch' }) });
  await expect(first.locator('[data-chip="status"]')).toBeVisible();
  await expect(first.locator('[data-chip="receipt"]')).toHaveText(/^fake · \d+ tool calls$/);
  await expect(page.getByText('A long line of body text')).toHaveCount(0);

  // Unfold the long card (its header is the click target).
  await page.getByRole('heading', { name: UNBROKEN, exact: true }).click();
  await expect(page.getByText('A long line of body text')).toBeVisible();
  await holdsLayout(page, '/pod/:id with a body open');

  // The ask: its options with a consequence showing, then the confirm row, from the keyboard.
  const ask = page.getByRole('article').filter({ has: page.getByRole('group', { name: 'Options' }) });
  await expect(ask).toBeVisible();
  await ask.getByRole('button', { name: /Merge now/ }).focus();
  await expect(ask.getByText('The agent merges on its next run.')).toBeVisible();
  await holdsLayout(page, '/pod/:id with the ask and a consequence showing');
  await ask.focus();
  await page.keyboard.press('a');
  await expect(page.getByRole('group', { name: 'Confirm ruling' })).toBeVisible();
  await holdsLayout(page, '/pod/:id with the confirm row');

  await page.getByRole('textbox', { name: 'Message' }).fill(`${UNBROKEN} ${UNBROKEN}`);
  await holdsLayout(page, '/pod/:id with a long draft in the composer');
});

test('the needs-you filter keeps its layout, with an ask, and when nothing needs you', async ({ page, pod }) => {
  await pod.agent.post(askInput(ASK_SUMMARY, { userIds: [pod.you.user.id] }));
  await pod.agent.post(askInput('Rename the billing table?', { userIds: [pod.other.user.id] }));
  await openPod(page, pod);
  const toggle = page.getByRole('button', { name: /^Needs you · / });
  await expect(toggle).toHaveText('Needs you · 1');
  await holdsLayout(page, '/pod/:id unfiltered');

  await page.keyboard.press('g');
  await page.keyboard.press('n');
  await expect(toggle).toHaveAttribute('aria-pressed', 'true');
  await expect(page.getByRole('article')).toHaveCount(1);
  await holdsLayout(page, '/pod/:id?filter=needs-you with one ask');

  await page.keyboard.press('a');
  await page.keyboard.press('Enter');
  await expect(page.getByText('Nothing needs you')).toBeVisible();
  await holdsLayout(page, '/pod/:id?filter=needs-you with nothing left');
});

test('/ keeps its layout signed out, signed in, and with a failure showing', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('link', { name: 'Sign in with GitHub to start a pod' })).toBeVisible();
  await holdsLayout(page, '/ signed out');

  await signIn(page, await person('Ada'));
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Create' })).toBeVisible();
  await holdsLayout(page, '/ signed in');

  // The longest name the schema allows, unbroken, in the field; then a refusal from the server shows inline.
  await page.getByLabel('Pod name').fill(UNBROKEN.slice(0, 80));
  await holdsLayout(page, '/ with a long name');
  await page.route('**/v1/pods', (route) => route.fulfill({ status: 400, contentType: 'application/json', body: JSON.stringify({ code: 'invalid_input', message: `That name does not work: ${UNBROKEN}` }) }));
  await page.getByRole('button', { name: 'Create' }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  await holdsLayout(page, '/ with a failure');
});

test('/invite/:token keeps its layout signed out, signed in, and when the link is spent', async ({ page }) => {
  const ada = await person('Ada');
  const { id } = await api<{ id: string }>('POST', '/v1/pods', ada.token, { name: 'Launch room' });
  const { code } = await api<{ code: string }>('POST', `/v1/pods/${id}/invites`, ada.token, { maxUses: 1 });
  const agent = await fakeAgent(id, ada, 'Scout');
  try {
    const path = `/invite/${code}`;
    await page.goto(path);
    await expect(page.getByRole('link', { name: 'Sign in with GitHub to join' })).toBeVisible();
    await holdsLayout(page, '/invite/:token signed out');

    await signIn(page, await person('Cy'));
    await page.goto(path);
    await expect(page.getByRole('button', { name: 'Join' })).toBeVisible();
    await holdsLayout(page, '/invite/:token signed in');
    await page.getByRole('button', { name: 'Join' }).click();
    await expect(page).toHaveURL(new RegExp(`/pod/${id}$`));

    // The link is spent for the next person; the page says so.
    await signIn(page, await person('Di'));
    await page.goto(path);
    await page.getByRole('button', { name: 'Join' }).click();
    await expect(page.getByRole('alert')).toBeVisible();
    await holdsLayout(page, '/invite/:token spent');
  } finally {
    agent.close();
  }
});
