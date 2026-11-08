import { expect, test } from 'bun:test';

// The e2e-preview job's first step is `preview.sh need-secrets`. A missing secret must fail it: a skip that exits 0 would let the job pass untested.
const script = new URL('../preview.sh', import.meta.url).pathname;
const need = (env: Record<string, string>) => Bun.spawnSync(['bash', script, 'need-secrets'], { env: { PATH: '/usr/bin:/bin', ...env } });

test('need-secrets fails when the token or the account id is missing or empty', () => {
  for (const env of [{}, { JM_CF_PREVIEW_TOKEN: 'x' }, { JM_CF_ACCOUNT_ID: 'y' }, { JM_CF_PREVIEW_TOKEN: '', JM_CF_ACCOUNT_ID: '' }]) {
    const r = need(env);
    expect(r.exitCode).not.toBe(0);
    expect(r.stderr.toString()).toContain('JM_CF_PREVIEW_TOKEN and JM_CF_ACCOUNT_ID');
  }
});

test('need-secrets passes when both are set', () => {
  expect(need({ JM_CF_PREVIEW_TOKEN: 'x', JM_CF_ACCOUNT_ID: 'y' }).exitCode).toBe(0);
});
