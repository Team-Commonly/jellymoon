// Stopping a run from the page. The fake agent's run is going from the moment the fixture builds it; the pod owner sees it
// working, presses Stop (a cookie POST), and the line says stopped only once the pod's run.ended reaches the page.
// Who is offered Stop is covered by unit tests: the fixture's socket bridge always carries the owner's token, so the hello
// of a spec page names the owner whoever the cookie is.
import { runOutcomeOf } from './pod';
import { expect, openPod, signIn, test } from './fixtures';

const STOP = "Stop Scout's run";

test('the pod owner sees a going run, presses Stop, and the line says stopped when the pod says the run ended', async ({ page, pod }) => {
  await signIn(page, pod.you);
  await openPod(page, pod);
  const runs = page.getByRole('list', { name: 'Runs' });
  await expect(runs).toContainText('Scout is working on a run.');
  expect(await runOutcomeOf(pod.id, pod.you.token, pod.agent.runId)).toBeNull();

  // Hold the pod's frames back: the button asks, and the line must not change until the pod's own answer is let through.
  pod.bridge.hold();
  await page.getByRole('button', { name: STOP }).click();
  await expect.poll(() => pod.bridge.sent('run.ended', (d) => d.runId === pod.agent.runId), { message: 'the pod ended the run' }).toBe(true);
  await expect(runs).toContainText('Scout is working on a run.');
  await expect(page.getByRole('button', { name: STOP })).toBeDisabled();

  pod.bridge.release();
  await expect(runs).toContainText("Scout's run stopped.");
  await expect(page.getByRole('button', { name: /^Stop/ })).toHaveCount(0);
  expect(await runOutcomeOf(pod.id, pod.you.token, pod.agent.runId)).toBe('cancelled');
});

test('a run that ended shows as stopped again after a reload, from the pod\'s replay', async ({ page, pod }) => {
  await signIn(page, pod.you);
  await openPod(page, pod);
  await page.getByRole('button', { name: STOP }).click();
  const runs = page.getByRole('list', { name: 'Runs' });
  await expect(runs).toContainText("Scout's run stopped.");
  await openPod(page, pod, () => page.reload());
  await expect(runs).toContainText("Scout's run stopped.");
  await expect(runs).not.toContainText('is working');
});
