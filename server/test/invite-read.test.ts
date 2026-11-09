import { expect, test } from 'bun:test';
import { hashToken } from '../src/ids';
import { migrate } from '../src/store';
import { hello } from '../src/views';
import { world } from './world';

// The pod's name in hello, the signed-out invite read and GET /v1/session, on the pod-state server.
const usesOf = async (w: Awaited<ReturnType<typeof world>>, code: string) =>
  (await w.db.rows<{ u: number; e: number }>('select uses_left as u, expires_at as e from invites where hash = $1', [hashToken(code)]))[0];

test('hello carries the pod\'s name', async () => {
  const w = await world();
  expect(hello(w.rt.state, w.rt.meta, w.olivia.user).podName).toBe('p');
});

test('an invite read needs no token and returns exactly the pod name, the inviter\'s name and the expiry, spending nothing', async () => {
  const w = await world();
  const inv = (await w.call('POST', `/v1/pods/${w.pod.id}/invites`, w.olivia.token, { maxUses: 1 })).json;
  const before = await usesOf(w, inv.code);
  for (let i = 0; i < 3; i++) {
    const read = await w.call('GET', `/v1/invites/${inv.code}`);
    expect(read).toEqual({ status: 200, json: { podName: 'p', inviterName: 'olivia', expiresAt: inv.expiresAt } });
  }
  expect(await usesOf(w, inv.code)).toEqual(before);
});

test('unknown, used, expired and inviter-less codes all get the same 404', async () => {
  const w = await world();
  const used = (await w.call('POST', `/v1/pods/${w.pod.id}/invites`, w.olivia.token, { maxUses: 1 })).json;
  const cara = (await w.call('POST', '/__test/session', undefined, { name: 'cara' })).json;
  expect((await w.call('POST', `/v1/invites/${used.code}/accept`, cara.token)).status).toBe(200);
  const expiring = (await w.call('POST', `/v1/pods/${w.pod.id}/invites`, w.olivia.token, { ttlSeconds: 60 })).json;
  const old = (await w.call('POST', `/v1/pods/${w.pod.id}/invites`, w.olivia.token, {})).json;
  await w.db.rows('update invites set inviter = null where hash = $1', [hashToken(old.code)]); // an invite from before inviters were kept
  await w.hub.setClock(w.hub.now() + 61_000);
  const answers = await Promise.all([`inv_${'A'.repeat(22)}`, used.code, expiring.code, old.code].map((c) => w.call('GET', `/v1/invites/${c}`)));
  for (const a of answers) expect(a).toEqual({ status: 404, json: { code: 'not_found', message: 'No such invite.' } });
});

test('the inviter column is added to an invites table that predates it', async () => {
  const w = await world();
  await w.db.exec('alter table invites drop column inviter');
  await migrate(w.db);
  const inv = (await w.call('POST', `/v1/pods/${w.pod.id}/invites`, w.olivia.token, {})).json;
  expect((await w.call('GET', `/v1/invites/${inv.code}`)).json.inviterName).toBe('olivia');
});

test('GET /v1/session is 200 with the user signed in and 200 with none signed out; GET /v1/me keeps its 401', async () => {
  const w = await world();
  expect(await w.call('GET', '/v1/session')).toEqual({ status: 200, json: {} });
  expect(await w.call('GET', '/v1/session', 'jmu_nope')).toEqual({ status: 200, json: {} });
  expect(await w.call('GET', '/v1/session', w.olivia.token)).toEqual({ status: 200, json: { user: w.olivia.user } });
  expect((await w.call('GET', '/v1/me')).status).toBe(401);
});
