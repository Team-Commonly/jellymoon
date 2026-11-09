// /device?code=WDJB-MJHT: the page the daemon's verification URL names. Never approves on load: a person presses Approve.
import { useRef, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { ApiError, signInHref } from './http';
import type { Http } from './http';

const SESSION_ENDED = 'Your session ended.';

function failure(e: unknown): string {
  if (!(e instanceof ApiError)) return 'Could not approve: the server could not be reached. Try again.';
  if (e.status === 404) return 'No device is waiting for that code. Check it against your terminal.';
  if (e.status === 410) return 'This code was already approved.';
  if (e.status === 400) return 'This code expired or is not a device code. Start the sign-in again in your terminal.';
  if (e.status === 401) return SESSION_ENDED;
  if (e.status === 403) return 'The server refused this approval. Open the link your terminal printed, on the address it shows.';
  return `Could not approve: the server returned an error (${e.status}). Try again.`;
}

export function DevicePage({ http, signedIn }: { http: Http; signedIn: boolean }) {
  const [params] = useSearchParams();
  const given = params.get('code') ?? '';
  const sending = useRef(false);
  const [code, setCode] = useState(given);
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState(false);
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
  if (done) return <section><h2>Device approved</h2><p>Your terminal is signed in. You can close this page.</p></section>;

  const approve = async () => {
    if (sending.current || !code.trim()) return;
    sending.current = true;
    setBusy(true);
    setError(null);
    try {
      await http.approveDevice(code.trim());
      setDone(true);
    } catch (e) {
      setError(failure(e));
      sending.current = false;
      setBusy(false);
    }
  };

  return (
    <section>
      <h2>Sign in your device</h2>
      <label>Device code <input value={code} onChange={(e) => setCode(e.target.value)} autoComplete="off" /></label>
      <button type="button" disabled={busy} onClick={approve}>Approve</button>
      {error && <p role="alert">{error}{error === SESSION_ENDED && <> <a href={back}>Sign in again</a></>}</p>}
    </section>
  );
}
