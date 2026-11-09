import { createRoot } from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import './app.css';
import { App } from './App';
import type { Connect } from './store';

// The pod socket. Cookie sign-in for the browser socket is a separate server unit; until it lands this connects
// without credentials and the pod refuses it.
const connect = (podId: string): Connect => (h) => {
  const ws = new WebSocket(`${location.origin.replace(/^http/, 'ws')}/v1/pods/${encodeURIComponent(podId)}/ws`);
  ws.onopen = () => h.open();
  ws.onmessage = (e) => h.message(String(e.data));
  ws.onclose = () => h.close();
  return { send: (t) => ws.send(t), close: () => ws.close() };
};

createRoot(document.getElementById('root')!).render(
  <BrowserRouter>
    <App connect={connect} />
  </BrowserRouter>,
);
