// /device?code=WDJB-MJHT: the page the daemon's verification URL names. Never approves on load: a person presses Approve, or
// "This wasn't me" to refuse a code they did not start, which stops the terminal that is waiting on it.
import { useRef, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { $defs } from '../../gen/ts/schemas.json';
import { ApiError, signInHref } from './http';
import type { Http } from './http';

type Route = 'approve' | 'deny';

const SESSION_ENDED = 'Your session ended.';
const NOT_A_CODE = "That isn't a valid code; check it against your terminal.";
const UNKNOWN = 'No device is waiting for that code. Check it against your terminal.';
// The shape the server enforces (case and hyphen forgiven, as it does), read from the generated schema so it is not copied here.
const CODE_SHAPE = new RegExp($defs.DeviceApproveRequest.properties.userCode.pattern);

function failure(e: unknown, route: Route): string {
  const verb = route === 'approve' ? 'approve' : 'refuse the code';
  if (!(e instanceof ApiError)) return `Could not ${verb}: the server could not be reached. Try again.`;
  if (e.status === 400 && e.code === 'invalid_input') return NOT_A_CODE;
  if (e.status === 401) return SESSION_ENDED;
  // Deny's refusals are approve's for the same code (unknown, expired, used); the page says only that no device waits on it.
  if (e.status === 404 || (route === 'deny' && (e.status === 400 || e.status === 410))) return UNKNOWN;
  if (e.status === 410) return 'This code was already used.';
  if (e.status === 400) return 'This code expired or is not a device code. Start the sign-in again in your terminal.';
  // The server's guess limit (ADR 0008 step 4): too many unknown, expired or used codes from this account or address.
  if (e.status === 429) return 'Too many tries. Wait a few minutes, then try again.';
  if (e.status === 403) return `The server refused this ${route === 'approve' ? 'approval' : 'request'}. Open the link your terminal printed, on the address it shows.`;
  return `Could not ${verb}: the server returned an error (${e.status}). Try again.`;
}

export function DevicePage({ http, signedIn }: { http: Http; signedIn: boolean }) {
  const [params] = useSearchParams();
  const given = params.get('code') ?? '';
  const sending = useRef(false);
  const [code, setCode] = useState(given);
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<{ route: Route; code: string } | null>(null);
  const [error, setError] = useState<string | null>(null);

  const back = signInHref(given ? `/device?code=${encodeURIComponent(given)}` : '/device');
  if (!signedIn) {
    return (
      <section>
        <h2>Sign in your device</h2>
        {given && <p>Your device code is <code>{given}</code>.</p>}
        <a href={back}>Sign in with GitHub to continue</a>
      </section>
    );
  }
  if (done?.route === 'approve') return <section><h2>Device approved</h2><p>Your terminal is signed in. You can close this page.</p></section>;
  if (done?.route === 'deny') {
    return <section><h2>Code refused</h2><p>{`The code ${done.code} will not sign anyone in, and the terminal that asked for it will stop waiting. You can close this page.`}</p></section>;
  }

  const send = async (route: Route) => {
    const typed = code.trim();
    if (sending.current || !typed) return;
    if (!CODE_SHAPE.test(typed)) return setError(NOT_A_CODE);
    sending.current = true;
    setBusy(true);
    setError(null);
    try {
      await (route === 'approve' ? http.approveDevice(typed) : http.denyDevice(typed));
      setDone({ route, code: typed });
    } catch (e) {
      setError(failure(e, route));
      sending.current = false;
      setBusy(false);
    }
  };

  return (
    <section>
      <h2>Sign in your device</h2>
      <label>Device code <input value={code} onChange={(e) => setCode(e.target.value)} autoComplete="off" /></label>
      <button type="button" disabled={busy} onClick={() => send('approve')}>Approve</button>
      <button type="button" disabled={busy} onClick={() => send('deny')}>This wasn't me</button>
      {error && <p role="alert">{error}{error === SESSION_ENDED && <> <a href={back}>Sign in again</a></>}</p>}
    </section>
  );
}
