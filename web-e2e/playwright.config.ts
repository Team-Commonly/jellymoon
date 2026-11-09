// The browser specs (docs/development/browser-specs.md): every spec at a phone and a desktop width, Chromium only, against the
// pod-state server global-setup starts. Browsers are installed in CI only; `make e2e-web` builds the client and runs these.
import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: '.',
  testMatch: '*.spec.ts',
  globalSetup: './global-setup.ts',
  outputDir: './test-results',
  reporter: [['list'], ['html', { open: 'never', outputFolder: './report' }]],
  fullyParallel: false,
  workers: 1,
  retries: 0,
  forbidOnly: true,
  timeout: 30_000,
  globalTimeout: 300_000,
  expect: { timeout: 5_000 },
  use: { browserName: 'chromium', trace: 'retain-on-failure', screenshot: 'only-on-failure' },
  projects: [
    { name: 'phone-390', use: { viewport: { width: 390, height: 844 } } },
    { name: 'desktop-1440', use: { viewport: { width: 1440, height: 900 } } },
  ],
});
