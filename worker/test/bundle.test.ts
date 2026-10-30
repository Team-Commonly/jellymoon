import { expect, test } from 'bun:test';

// F13: the production bundle has no /__test handler at all. Bundle the production entry and read the result.
test('the production entry bundles with no test route, clock override or test bearer', async () => {
  const built = await Bun.build({ entrypoints: [new URL('../src/index.ts', import.meta.url).pathname], target: 'browser', minify: false });
  expect(built.success).toBe(true);
  const text = await built.outputs[0].text();
  // '/_in/evict' is the eviction's route inside the pod object and 'evict' its handler's name: both belong to the test pod object only.
  // '/_in/discard', '/_in/instance' and '/_in/stored-alarm' throw the object away, read its instance id and read its stored alarm; 'alarm_at',
  // 'alarm_was', 'alarm_clock' and 'woke:' are the keys the discard and the alarm handler keep. All of them: test pod object only.
  for (const needle of ['__test', 'TEST_BEARER', 'TestPodDO', 'BOOT_SWEEP', '/_in/evict', 'evict', '/_in/discard', '/_in/instance', '/_in/stored-alarm', 'alarm_at', 'alarm_was', 'alarm_clock', 'woke:']) expect(text.includes(needle)).toBe(false);
  expect(text.includes('PodDO')).toBe(true);
});

test('the test entry does carry the test routes', async () => {
  const built = await Bun.build({ entrypoints: [new URL('../src/test/index.ts', import.meta.url).pathname], target: 'browser', loader: { '.sql': 'text' } });
  expect(built.success).toBe(true);
  const text = await built.outputs[0].text();
  // The /__test routes with a pod id are regular expressions, so their text in the bundle has escaped slashes.
  for (const needle of ['/__test/clock', '/_in/evict', 'evict', '__test\\/discard', '__test\\/instance', '__test\\/woke', '/_in/discard', '/_in/instance', '__test\\/stored-alarm', '/_in/stored-alarm', 'alarm_at', 'alarm_was', 'alarm_clock', 'woke:']) expect(text.includes(needle)).toBe(true);
});
