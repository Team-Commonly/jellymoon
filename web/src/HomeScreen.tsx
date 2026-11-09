// /. Signed out: what Jellymoon is and the way in. Signed in: make a pod. Never creates on load: a person presses Create.
import { useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { ApiError, signInHref } from './http';
import type { Http } from './http';

/** CreatePodRequest.name: 1 to 80 characters (schema/pod-api.json). */
const NAME_MAX = 80;

function failure(e: unknown): string {
  if (e instanceof ApiError) return `Could not create the pod: ${e.message}`;
  return 'Could not create the pod: the server could not be reached. Try again.';
}

export function HomePage({ http, signedIn }: { http: Http; signedIn: boolean }) {
  const navigate = useNavigate();
  const sending = useRef(false);
  const [name, setName] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!signedIn) {
    return (
      <section>
        <h2>Jellymoon</h2>
        <p>A shared room where you and your coding agents work, and where agents ask you before they act.</p>
        <a href={signInHref('/')}>Sign in with GitHub to start a pod</a>
      </section>
    );
  }

  const trimmed = name.trim();
  const create = async (e: FormEvent) => {
    e.preventDefault();
    if (sending.current || !trimmed) return;
    sending.current = true;
    setBusy(true);
    setError(null);
    try {
      const pod = await http.createPod(trimmed);
      navigate(`/pod/${encodeURIComponent(pod.id)}`);
    } catch (err) {
      setError(failure(err));
      sending.current = false;
      setBusy(false);
    }
  };

  return (
    <section>
      <h2>New pod</h2>
      <form onSubmit={create}>
        <label>Pod name <input value={name} onChange={(e) => setName(e.target.value)} maxLength={NAME_MAX} autoComplete="off" /></label>
        <button type="submit" disabled={busy || !trimmed}>Create</button>
      </form>
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
