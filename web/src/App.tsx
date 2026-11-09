// The shell: one layout with a top bar and a router. The pod screen puts its needs-you toggle into the bar's slot.
import { useState } from 'react';
import { Outlet, Route, Routes, useParams } from 'react-router-dom';
import type { Http } from './http';
import { InvitePage } from './InviteScreen';
import { PodPage } from './PodScreen';
import type { Connect } from './store';

function Shell() {
  const { id } = useParams();
  const [slot, setSlot] = useState<HTMLElement | null>(null);
  return (
    <>
      <header>
        <strong>{id ? `Pod ${id}` : 'Jellymoon'}</strong>
        <span ref={setSlot} />
        <span> Not signed in </span>
        <button type="button">Sign in</button>
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
      <Route element={<Shell />}>
        <Route path="/pod/:id" element={<PodPage connect={connect} />} />
        <Route path="/invite/:token" element={<InvitePage http={http} signedIn={signedIn} />} />
        <Route path="*" element={<p>Nothing here.</p>} />
      </Route>
    </Routes>
  );
}
