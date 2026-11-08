// The shell: one layout with a top bar and a router.
import { Outlet, Route, Routes } from 'react-router-dom';
import { PodPage } from './PodScreen';
import type { Connect } from './store';

function Shell() {
  return (
    <>
      <header>
        <strong>Jellymoon</strong>
        <span> Not signed in </span>
        <button type="button">Sign in</button>
      </header>
      <main>
        <Outlet />
      </main>
    </>
  );
}

export function App({ connect }: { connect: (podId: string) => Connect }) {
  return (
    <Routes>
      <Route element={<Shell />}>
        <Route path="/pod/:id" element={<PodPage connect={connect} />} />
        <Route path="*" element={<p>Nothing here.</p>} />
      </Route>
    </Routes>
  );
}
