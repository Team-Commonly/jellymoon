// The Worker: authenticates against D1, checks the token kind and the pod, and hands the request to the pod's Durable Object
// with the caller in an internal header (any incoming copy is deleted first). It holds no pod state.
import { addAgentPod, addInvite, addPod, authenticate, createComputer, dropInvite, dropPod, dropRunToken, mintRunToken, podExists, podOfInvite, podOfRun, podsOfComputer } from './directory';
import type { Env } from './env';
import { hashToken, newId, newInviteCode } from './ids';
import { mcp } from './mcp';
import { check } from './validate';
import { bad, encodeWho, fail, parse, WHO_HEADER } from './wire';
import type { Body, Who } from './wire';

type Kind = Who['kind'];
const POD = '^/v1/pods/([^/]+)';
/** Which token kinds may call each pod route; the pod object does the rest. */
const POD_KINDS: Array<[string, RegExp, Kind[]]> = [
  ['GET', new RegExp(`${POD}/(me|feed|tasks)$`), ['user', 'run']],
  ['POST', new RegExp(`${POD}/tasks/[^/]+/(claim|release)$`), ['user', 'run']],
  ['PATCH', new RegExp(`${POD}/tasks/[^/]+$`), ['user', 'run']],
  ['POST', new RegExp(`${POD}/runs/[^/]+/stop$`), ['user', 'run']],
  ['POST', new RegExp(`${POD}/messages$`), ['user', 'run']],
  ['POST', new RegExp(`${POD}/agents$`), ['computer']],
  ['GET', new RegExp(`${POD}/ws$`), ['user', 'computer']],
];

const stub = (env: Env, podId: string): DurableObjectStub => env.POD.get(env.POD.idFromName(podId));

/** A call to a pod's internal route. */
export function internal(env: Env, podId: string, name: string, body: Body): Promise<Response> {
  return stub(env, podId).fetch(`https://pod/_in/${name}`, { method: 'POST', body: JSON.stringify(body) });
}

function forward(env: Env, req: Request, podId: string, who: Who): Promise<Response> {
  const headers = new Headers(req.headers);
  headers.delete(WHO_HEADER);
  headers.set(WHO_HEADER, encodeWho(who));
  return stub(env, podId).fetch(new Request(req, { headers }));
}

type Guarded = { who: Who } | Response;
async function guard(env: Env, req: Request, kinds: readonly Kind[]): Promise<Guarded> {
  const who = await authenticate(env.DB, req.headers.get('Authorization'));
  if (who === null) return fail('unauthorized', 'Send a valid bearer token.');
  return kinds.includes(who.kind) ? { who } : fail('forbidden', 'This kind of token may not call this route.');
}

async function podCall(env: Env, req: Request, kinds: readonly Kind[], podId: string, fn: (who: Who) => Promise<Response>): Promise<Response> {
  const g = await guard(env, req, kinds);
  if (g instanceof Response) return g;
  if (g.who.kind === 'run' && g.who.podId !== podId) return fail('forbidden', 'This run token belongs to another pod.');
  if (!(await podExists(env.DB, podId))) return fail('not_found', 'No such pod.', podId);
  return fn(g.who);
}

/** `def` names the request definition in gen/ the body is checked against; null for a route that takes no body. */
async function setup(env: Env, req: Request, kinds: readonly Kind[], def: string | null, fn: (who: Who, body: Body) => Promise<Response>): Promise<Response> {
  const g = await guard(env, req, kinds);
  if (g instanceof Response) return g;
  const body = await parse(req);
  if (body === null) return bad('The body must be a JSON object.');
  const problem = def === null ? null : check(def, body);
  return problem === null ? fn(g.who, body) : bad(problem);
}

const userOf = (who: Who) => (who as Extract<Who, { kind: 'user' }>).actor;
const computerOf = (who: Who) => who as Extract<Who, { kind: 'computer' }>;

async function createPod(env: Env, who: Who, body: Body): Promise<Response> {
  const name = body.name as string;
  const id = newId('pod_');
  await addPod(env.DB, id);
  const res = await internal(env, id, 'create', { podId: id, name, ownerId: userOf(who).id, ownerName: userOf(who).displayName });
  if (!res.ok) await dropPod(env.DB, id);
  return res;
}

/** The code is shown once, in this reply. D1 and the pod keep its SHA-256, and the join link is built from PUBLIC_URL. */
async function createInvite(env: Env, podId: string, who: Who, body: Body, origin: string): Promise<Response> {
  const code = newInviteCode();
  const hash = await hashToken(code);
  await addInvite(env.DB, hash, podId);
  const res = await internal(env, podId, 'invite', { ...body, code, hash, userId: userOf(who).id, base: env.PUBLIC_URL ?? origin });
  if (!res.ok) await dropInvite(env.DB, hash);
  return res;
}

async function acceptInvite(env: Env, code: string, who: Who): Promise<Response> {
  const hash = await hashToken(code);
  const podId = await podOfInvite(env.DB, hash);
  if (podId === null) return fail('not_found', 'No such invite.');
  return internal(env, podId, 'accept', { hash, userId: userOf(who).id, name: userOf(who).displayName });
}

async function registerAgent(env: Env, req: Request, podId: string, who: Who): Promise<Response> {
  const res = await forward(env, req, podId, who);
  if (res.ok) await addAgentPod(env.DB, computerOf(who).computerId, podId);
  return res;
}

/** Wake ids are per pod: ask each pod where this computer hosts an agent whether the wake is its own. */
async function startRun(env: Env, who: Who, body: Body, origin: string): Promise<Response> {
  const wakeId = body.wakeId as string;
  const computerId = computerOf(who).computerId;
  const pods = await podsOfComputer(env.DB, computerId);
  const has = await Promise.all(pods.map(async (p) => ((await (await internal(env, p, 'has-wake', { wakeId, computerId })).json()) as { has: boolean }).has));
  const podId = pods[has.indexOf(true)];
  if (podId === undefined) return fail('not_found', 'No such wake, or it was already claimed.', wakeId);
  const runId = newId('run_');
  const runToken = await mintRunToken(env.DB, runId, podId);
  const res = await internal(env, podId, 'run-claim', { wakeId, computerId, runId });
  if (!res.ok) { await dropRunToken(env.DB, runId); return res; }
  const claimed = (await res.json()) as { runId: string; brief: string; expiresAt: string };
  return Response.json({ ...claimed, runToken, mcpUrl: `${env.PUBLIC_URL ?? origin}/mcp` });
}

async function endRun(env: Env, who: Who, body: Body, runId: string): Promise<Response> {
  // The core's run_end has no place for the harness version, so it is refused rather than dropped.
  if (body.harnessVersion !== undefined) return bad('body.harnessVersion: this server does not store a harness version yet');
  const outcome = body.outcome as string;
  const podId = await podOfRun(env.DB, runId);
  if (podId === null) return fail('not_found', 'No such run.', runId);
  return internal(env, podId, 'run-end', { runId, outcome, computerId: computerOf(who).computerId });
}

async function newComputer(env: Env, who: Who, body: Body): Promise<Response> {
  return Response.json(await createComputer(env.DB, userOf(who).id, body.name as string, body.os as string, body.arch as string));
}

/** Every public route. The test entry wraps this; the production entry is this alone. */
export async function handle(req: Request, env: Env): Promise<Response> {
  const url = new URL(req.url);
  const path = url.pathname;
  const m = req.method;
  if (path === '/healthz') return new Response('ok');
  if (m === 'POST' && path === '/v1/pods') return setup(env, req, ['user'], 'CreatePodRequest', (w, b) => createPod(env, w, b));
  if (m === 'POST' && path === '/v1/computers') return setup(env, req, ['user'], 'RegisterComputerRequest', (w, b) => newComputer(env, w, b));
  if (m === 'POST' && path === '/v1/runs') return setup(env, req, ['computer'], 'ClaimRunRequest', (w, b) => startRun(env, w, b, url.origin));
  if (m === 'POST' && path === '/mcp') {
    const g = await guard(env, req, ['run', 'user']);
    return g instanceof Response ? g : mcp(req, url.origin, g.who, (r) => handle(r, env));
  }
  const run = /^\/v1\/runs\/([^/]+)\/end$/.exec(path);
  if (m === 'POST' && run) return setup(env, req, ['computer'], 'EndRunRequest', (w, b) => endRun(env, w, b, run[1]));
  const inv = /^\/v1\/invites\/([^/]+)\/accept$/.exec(path);
  if (m === 'POST' && inv) return setup(env, req, ['user'], null, (w) => acceptInvite(env, inv[1], w));
  const create = new RegExp(`${POD}/invites$`).exec(path);
  if (m === 'POST' && create) {
    return podCall(env, req, ['user'], create[1], async (w) => {
      const body = await parse(req);
      const problem = body === null ? 'The body must be a JSON object.' : check('CreateInviteRequest', body);
      return body === null || problem !== null ? bad(problem ?? '') : createInvite(env, create[1], w, body, url.origin);
    });
  }
  if (m === 'GET' && new RegExp(`${POD}/runs/[^/]+/tool-calls$`).test(path)) return fail('not_found', 'Evidence is not served by this server.');
  for (const [method, re, kinds] of POD_KINDS) {
    const hit = method === m ? re.exec(path) : null;
    if (hit === null) continue;
    const podId = hit[1];
    if (re.source.endsWith('/agents$')) return podCall(env, req, kinds, podId, (w) => registerAgent(env, req, podId, w));
    return podCall(env, req, kinds, podId, (w) => forward(env, req, podId, w));
  }
  return fail('not_found', 'No such route.');
}
