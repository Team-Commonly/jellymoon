// Binds one pod's store to a component: starts the socket on mount, stops it on unmount, and re-renders on change.
import { useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { createPodStore } from './store';
import type { Connect } from './store';

export function usePod(podId: string, connect: (podId: string) => Connect) {
  const connectRef = useRef(connect);
  const [store] = useState(() => createPodStore({ podId, connect: connectRef.current(podId) }));
  const state = useSyncExternalStore(store.subscribe, store.getState);
  useEffect(() => {
    store.start();
    return () => store.stop();
  }, [store]);
  return { store, state };
}
