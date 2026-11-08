import { createRoot } from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import { App } from './App';
import type { Connect } from './store';

const connect = (podId: string): Connect => () => {
  throw new Error(`no socket for ${podId}`);
};

createRoot(document.getElementById('root')!).render(
  <BrowserRouter>
    <App connect={connect} />
  </BrowserRouter>,
);
