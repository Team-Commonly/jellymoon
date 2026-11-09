// Binds one pod's store to a component: starts the socket on mount, stops it on unmount, and re-renders on change.
import { useEffect, useRef, useState, useSyncExternalStore } from 'react';
import type { PodAccess } from './http';
import { createPodStore } from './store';
import type { Connect } from './store';

/** `access` answers why a socket closed before its hello: a refused pod stops the store for good, a member's drop retries. */
export function usePod(podId: string, connect: (podId: string) => Connect, access: (podId: string) => Promise<PodAccess>) {
  const connectRef = useRef(connect);
  const accessRef = useRef(access);
  const [store] = useState(() => createPodStore({
    podId,
    connect: connectRef.current(podId),
    check: async () => {
      const a = await accessRef.current(podId);
      return a === 'member' ? null : a;
    },
  }));
  const state = useSyncExternalStore(store.subscribe, store.getState);
  useEffect(() => {
    store.start();
    return () => store.stop();
  }, [store]);
  return { store, state };
}
