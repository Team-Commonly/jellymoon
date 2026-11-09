import { createRoot } from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import './app.css';
import { App } from './App';
import { bootSignedIn, browserHttp } from './http';
import type { Connect } from './store';

// The pod socket. The browser sends its session cookie on the upgrade.
const connect = (podId: string): Connect => (h) => {
  const ws = new WebSocket(`${location.origin.replace(/^http/, 'ws')}/v1/pods/${encodeURIComponent(podId)}/ws`);
  ws.onopen = () => h.open();
  ws.onmessage = (e) => h.message(String(e.data));
  ws.onclose = () => h.close();
  return { send: (t) => ws.send(t), close: () => ws.close() };
};

// Signed in or not is what GET /v1/session says for the session cookie; an unreachable or silent server counts as signed out.
void bootSignedIn(browserHttp).then((signedIn) => {
  createRoot(document.getElementById('root')!).render(
    <BrowserRouter>
      <App connect={connect} http={browserHttp} signedIn={signedIn} />
    </BrowserRouter>,
  );
});
