// The spec fixture: a fresh pod per test (an owner, a second member, the fake agent) and a page whose pod socket reaches the
// real server through a bridge. The bridge exists because main has no browser session yet: the server takes the token only in
// Authorization on the upgrade, which a browser cannot set. Playwright takes the page's socket and the bridge opens the same URL
// from Node with the owner's token, relaying frames both ways unchanged. It also records them, and can hold the server's
// frames back, so a spec can show the page waits for the pod's answer rather than updating on its own.
import { expect, test as base } from '@playwright/test';
import type { Page, WebSocketRoute } from '@playwright/test';
import WebSocket from 'ws';
import type { Envelope } from '../gen/ts/protocol';
import { api, fakeAgent, person } from './pod';
import type { FakeAgent, Person } from './pod';

class Bridge {
  /** Frames the page sent, and frames the server sent (relayed or still held), across every connection. */
  readonly fromPage: Array<Envelope & { conn: number }> = [];
  readonly fromServer: Array<Envelope & { conn: number }> = [];
  connections = 0;
  private held: Array<() => void> | null = null;
  private ups: WebSocket[] = [];

  constructor(private token: string) {}

  attach(route: WebSocketRoute): void {
    const conn = ++this.connections;
    const up = new WebSocket(route.url(), { headers: { authorization: `Bearer ${this.token}` } });
    this.ups.push(up);
    const queued: string[] = [];
    up.on('open', () => queued.splice(0).forEach((t) => up.send(t)));
    up.on('error', () => void route.close({ code: 1011, reason: 'the bridge could not reach the pod' }));
    up.on('close', () => void route.close());
    up.on('message', (d: unknown) => {
      const text = String(d);
      this.fromServer.push({ ...(JSON.parse(text) as Envelope), conn });
      const relay = () => { try { route.send(text); } catch { /* the page navigated away */ } };
      if (this.held) this.held.push(relay);
      else relay();
    });
    route.onMessage((m) => {
      const text = String(m);
      this.fromPage.push({ ...(JSON.parse(text) as Envelope), conn });
      if (up.readyState === WebSocket.OPEN) up.send(text);
      else queued.push(text);
    });
    route.onClose(() => up.close());
  }

  /** From now on the server's frames wait here until release(). */
  hold(): void {
    this.held ??= [];
  }

  release(): void {
    const h = this.held ?? [];
    this.held = null;
    h.forEach((relay) => relay());
  }

  /** Whether the server has sent `type` on connection `conn` (the latest by default), relayed to the page or still held. */
  sent(type: string, match: (data: Record<string, unknown>) => boolean = () => true, conn = this.connections): boolean {
    return this.fromServer.some((f) => f.conn === conn && f.type === type && match(f.data));
  }

  close(): void {
    this.ups.forEach((u) => u.close());
  }
}

/** Signs the page in the way GitHub's callback does: the session cookie holds the person's user token (BSESS). */
export async function signIn(page: Page, who: Person): Promise<void> {
  const url = process.env.JM_WEB_E2E_BASE ?? '';
  await page.context().addCookies([{ name: 'jm_session', value: who.token, url, httpOnly: true, sameSite: 'Lax' }]);
}

export interface Pod { id: string; you: Person; other: Person; agent: FakeAgent; bridge: Bridge }

/** Opens the pod page (or runs `go`, a reload) and waits until the page's new socket has had hello and has subscribed. */
export async function openPod(page: Page, pod: Pod, go: () => Promise<unknown> = () => page.goto(`/pod/${pod.id}`)): Promise<void> {
  const before = pod.bridge.connections;
  await go();
  await expect.poll(() => pod.bridge.connections, { message: 'the page opened its pod socket' }).toBeGreaterThan(before);
  const conn = pod.bridge.connections;
  await expect.poll(() => pod.bridge.sent('hello') && pod.bridge.fromPage.some((f) => f.conn === conn && f.type === 'subscribe'), { message: 'hello and subscribe' }).toBe(true);
}

export const test = base.extend<{ pod: Pod }>({
  baseURL: async ({}, use) => {
    const b = process.env.JM_WEB_E2E_BASE;
    if (!b) throw new Error('JM_WEB_E2E_BASE is unset: global-setup did not start the server');
    await use(b);
  },
  pod: async ({ page }, use) => {
    const you = await person('Ada');
    const other = await person('Bo');
    const { id } = await api<{ id: string }>('POST', '/v1/pods', you.token, { name: 'Launch room' });
    const { code } = await api<{ code: string }>('POST', `/v1/pods/${id}/invites`, you.token, { maxUses: 1 });
    await api('POST', `/v1/invites/${code}/accept`, other.token);
    const agent = await fakeAgent(id, you, 'Scout');
    const bridge = new Bridge(you.token);
    await page.routeWebSocket(/\/v1\/pods\/[^/]+\/ws$/, (route) => bridge.attach(route));
    await use({ id, you, other, agent, bridge });
    bridge.close();
    agent.close();
  },
});

export { expect };
