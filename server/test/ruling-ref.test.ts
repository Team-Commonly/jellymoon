import { expect, test } from 'bun:test';
import { websocket } from 'hono/bun';
import { setup } from './world';

// A refused ruling names the decision it was about, so the web store can put the error on that card.
const UNKNOWN = 'dec_01ARZ3NDEKTSV4RRFFQ69G5FAZ';

/** Opens a real socket, sends one frame once hello has arrived, and resolves with the first error frame's data. */
async function errorFor(type: string, data: unknown): Promise<Record<string, unknown>> {
  const s = await setup();
  const token = (await s.call('POST', '/__test/session', undefined, { name: 'olivia' })).json.token as string;
  const podId = (await s.call('POST', '/v1/pods', token, { name: 'p' })).json.id as string;
  const srv = Bun.serve({ port: 0, fetch: s.app.fetch, websocket });
  const ws = new WebSocket(`ws://127.0.0.1:${srv.port}/v1/pods/${podId}/ws`, { headers: { Authorization: `Bearer ${token}` } } as unknown as string[]);
  return new Promise((resolve) => {
    ws.onmessage = (e) => {
      const f = JSON.parse(String(e.data)) as { type: string; data: Record<string, unknown> };
      if (f.type === 'hello') ws.send(JSON.stringify({ v: 1, type, podId, data }));
      if (f.type === 'error') { ws.close(); resolve(f.data); }
    };
  });
}

test('a ruling the core refuses comes back as an error frame whose ref is the decision id', async () => {
  expect(await errorFor('decision.rule', { decisionId: UNKNOWN, optionId: 'a' })).toEqual({ code: 'not_found', message: 'No such decision.', ref: UNKNOWN });
});

test('an error frame that is not about a ruling carries no ref', async () => {
  expect(Object.keys(await errorFor('message.post', {}))).toEqual(['code', 'message']);
});
