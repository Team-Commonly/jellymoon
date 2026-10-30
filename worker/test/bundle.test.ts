import { expect, test } from 'bun:test';

// F13: the production bundle has no /__test handler at all. Bundle the production entry and read the result.
test('the production entry bundles with no test route, clock override or test bearer', async () => {
  const built = await Bun.build({ entrypoints: [new URL('../src/index.ts', import.meta.url).pathname], target: 'browser', minify: false });
  expect(built.success).toBe(true);
  const text = await built.outputs[0].text();
  // '/_in/evict' is the eviction's route inside the pod object and 'evict' its handler's name: both belong to the test pod object only.
  for (const needle of ['__test', 'TEST_BEARER', 'TestPodDO', 'BOOT_SWEEP', '/_in/evict', 'evict']) expect(text.includes(needle)).toBe(false);
  expect(text.includes('PodDO')).toBe(true);
});

test('the test entry does carry the test routes', async () => {
  const built = await Bun.build({ entrypoints: [new URL('../src/test/index.ts', import.meta.url).pathname], target: 'browser', loader: { '.sql': 'text' } });
  expect(built.success).toBe(true);
  const text = await built.outputs[0].text();
  for (const needle of ['/__test/clock', '/_in/evict', 'evict']) expect(text.includes(needle)).toBe(true);
});
