// /invite/:token. Never joins on load: a person presses Join. The invite code is never logged or put in another URL.
import { useRef, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { ApiError } from './http';
import type { Http } from './http';

const GONE = 'This invite has been used or has expired. Ask whoever sent it for a new one.';

function failure(e: unknown): string {
  if (e instanceof ApiError) return e.status === 410 ? GONE : `Could not join: the server returned an error (${e.status}). Try again.`;
  return 'Could not join: the server could not be reached. Try again.';
}

export function InvitePage({ http, signedIn }: { http: Http; signedIn: boolean }) {
  const { token = '' } = useParams();
  const navigate = useNavigate();
  const joining = useRef(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!signedIn) {
    const back = encodeURIComponent(`/invite/${token}`);
    return (
      <section>
        <h2>You have been invited to a pod</h2>
        <a href={`/sign-in?return=${back}`}>Sign in with GitHub to join</a>
      </section>
    );
  }

  const join = async () => {
    if (joining.current) return;
    joining.current = true;
    setBusy(true);
    setError(null);
    try {
      const { pod } = await http.acceptInvite(token);
      navigate(`/pod/${encodeURIComponent(pod.id)}`);
    } catch (e) {
      setError(failure(e));
      joining.current = false;
      setBusy(false);
    }
  };

  return (
    <section>
      <h2>You have been invited to a pod</h2>
      <button type="button" disabled={busy} onClick={join}>Join</button>
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
