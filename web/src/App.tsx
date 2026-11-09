// The shell: one layout with a top bar and a router. The pod screen puts its needs-you toggle into the bar's slot and its name in the title.
import { useRef, useState } from 'react';
import { Outlet, Route, Routes, useLocation, useNavigate, useParams } from 'react-router-dom';
import type { Actor } from '../../gen/ts/protocol';
import type { BarContext } from './bar';
import { DevicePage } from './DeviceScreen';
import { signInHref } from './http';
import type { Http } from './http';
import { HomePage } from './HomeScreen';
import { InvitePage } from './InviteScreen';
import { PodPage } from './PodScreen';
import type { Connect } from './store';

/** The name the bar shows: the display name, or the id when the display name is blank. On this server a display name starts as the GitHub login. */
const nameOf = (me: Actor): string => me.displayName.trim() || me.id;

/** Who is signed in, and Sign out. One logout at a time; a failure says so and leaves the person signed in. */
function SignedIn({ me, http, signedOut }: { me: Actor; http: Http; signedOut(): void }) {
  const sending = useRef(false);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const out = async () => {
    if (sending.current) return;
    sending.current = true;
    setBusy(true);
    setFailed(false);
    try {
      await http.logout();
      signedOut();
    } catch {
      sending.current = false;
      setBusy(false);
      setFailed(true);
    }
  };
  const name = nameOf(me);
  return (
    <>
      <span data-session="">
        <span data-who="" title={name}>{name}</span>
        <button type="button" onClick={() => void out()} disabled={busy}>Sign out</button>
      </span>
      {failed && <span role="alert">Could not sign out. Try again.</span>}
    </>
  );
}

// The right of the bar is the session: who is in, or the real sign-in that brings the person back to this very page.
function Shell({ me, http, signedOut }: { me: Actor | null; http: Http; signedOut(): void }) {
  const { id } = useParams();
  const here = useLocation();
  const [slot, setSlot] = useState<HTMLElement | null>(null);
  const [title, setTitle] = useState<string>();
  return (
    <>
      <header>
        <strong>{id ? (title ?? `Pod ${id}`) : 'Jellymoon'}</strong>
        <span ref={setSlot} />
        {me ? <SignedIn me={me} http={http} signedOut={signedOut} /> : <a href={signInHref(here.pathname + here.search)}>Sign in</a>}
      </header>
      <main>
        <Outlet context={{ slot, setTitle } satisfies BarContext} />
      </main>
    </>
  );
}

/**
 * `me` is who GET /v1/session named at boot. Signing out goes back to `/` signed out, in the page: the routes are keyed by who is
 * in, so every screen (a pod's socket included) unmounts and nothing from the signed-in session is left on screen.
 */
export function App({ connect, http, me: booted }: { connect: (podId: string) => Connect; http: Http; me: Actor | null }) {
  const navigate = useNavigate();
  const [me, setMe] = useState(booted);
  const signedIn = me !== null;
  const signedOut = () => { setMe(null); navigate('/'); };
  return (
    <Routes key={me?.id ?? ''}>
      <Route element={<Shell me={me} http={http} signedOut={signedOut} />}>
        <Route path="/" element={<HomePage http={http} signedIn={signedIn} />} />
        <Route path="/pod/:id" element={<PodPage connect={connect} http={http} />} />
        <Route path="/device" element={<DevicePage http={http} signedIn={signedIn} />} />
        <Route path="/invite/:token" element={<InvitePage http={http} signedIn={signedIn} />} />
        <Route path="*" element={<p>Nothing here.</p>} />
      </Route>
    </Routes>
  );
}
