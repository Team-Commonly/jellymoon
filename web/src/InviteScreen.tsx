// /invite/:token. Never joins on load: a person presses Join. The invite code is never logged; it appears only in same-origin URLs.
import { useEffect, useRef, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import type { InvitePreview } from '../../gen/ts/protocol';
import { ApiError, signInHref } from './http';
import type { Http } from './http';

const GONE = 'This invite has been used or has expired. Ask whoever sent it for a new one.';

function failure(e: unknown): string {
  if (e instanceof ApiError) return e.status === 410 ? GONE : `Could not join: the server returned an error (${e.status}). Try again.`;
  return 'Could not join: the server could not be reached. Try again.';
}

/** The invite read, before sign-in. A 404 or a failed read leaves null: the page still offers sign-in and Join. */
function usePreview(http: Http, token: string): InvitePreview | null {
  const [preview, setPreview] = useState<InvitePreview | null>(null);
  useEffect(() => {
    let live = true;
    setPreview(null); // another code's names never stay on screen while this one is read, or after its read fails
    http.readInvite(token).then((p) => { if (live) setPreview(p); }, () => {});
    return () => { live = false; };
  }, [http, token]);
  return preview;
}

export function InvitePage({ http, signedIn }: { http: Http; signedIn: boolean }) {
  const { token = '' } = useParams();
  const navigate = useNavigate();
  const joining = useRef(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const preview = usePreview(http, token);
  const heading = <h2>{preview ? `${preview.inviterName} invited you to ${preview.podName}` : 'You have been invited to a pod'}</h2>;

  if (!signedIn) {
    return (
      <section>
        {heading}
        <a href={signInHref(`/invite/${token}`)}>Sign in with GitHub to join</a>
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
      {heading}
      <button type="button" disabled={busy} onClick={join}>Join</button>
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
