// The shell: one layout with a top bar and a router. Screens come in later pull requests; /pod/:id is a placeholder.
import { Outlet, Route, Routes, useParams } from 'react-router-dom';

function Shell() {
  const { id } = useParams();
  return (
    <>
      <header>
        <strong>{id ? `Pod ${id}` : 'Jellymoon'}</strong>
        <span> Not signed in </span>
        <button type="button">Sign in</button>
      </header>
      <main>
        <Outlet />
      </main>
    </>
  );
}

function PodPlaceholder() {
  const { id } = useParams();
  return <p>Pod {id} will show here.</p>;
}

export function App() {
  return (
    <Routes>
      <Route element={<Shell />}>
        <Route path="/pod/:id" element={<PodPlaceholder />} />
        <Route path="*" element={<p>Nothing here.</p>} />
      </Route>
    </Routes>
  );
}
