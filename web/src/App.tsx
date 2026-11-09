// The shell: one layout with a top bar and a router. The pod screen puts its needs-you toggle into the bar's slot.
import { useState } from 'react';
import { Outlet, Route, Routes, useLocation, useParams } from 'react-router-dom';
import { DevicePage } from './DeviceScreen';
import { signInHref } from './http';
import type { Http } from './http';
import { HomePage } from './HomeScreen';
import { InvitePage } from './InviteScreen';
import { PodPage } from './PodScreen';
import type { Connect } from './store';

// The right of the bar is the session: who is in, or the real sign-in that brings the person back to this very page.
function Shell({ signedIn }: { signedIn: boolean }) {
  const { id } = useParams();
  const here = useLocation();
  const [slot, setSlot] = useState<HTMLElement | null>(null);
  return (
    <>
      <header>
        <strong>{id ? `Pod ${id}` : 'Jellymoon'}</strong>
        <span ref={setSlot} />
        {signedIn ? <span>Signed in</span> : <a href={signInHref(here.pathname + here.search)}>Sign in</a>}
      </header>
      <main>
        <Outlet context={slot} />
      </main>
    </>
  );
}

export function App({ connect, http, signedIn }: { connect: (podId: string) => Connect; http: Http; signedIn: boolean }) {
  return (
    <Routes>
      <Route element={<Shell signedIn={signedIn} />}>
        <Route path="/" element={<HomePage http={http} signedIn={signedIn} />} />
        <Route path="/pod/:id" element={<PodPage connect={connect} http={http} />} />
        <Route path="/device" element={<DevicePage http={http} signedIn={signedIn} />} />
        <Route path="/invite/:token" element={<InvitePage http={http} signedIn={signedIn} />} />
        <Route path="*" element={<p>Nothing here.</p>} />
      </Route>
    </Routes>
  );
}
