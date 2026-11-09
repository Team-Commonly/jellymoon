// The pod's name reaches this client only in the answer to creating a pod or accepting an invite; the pod socket's hello carries
// the id and no name. So the name is remembered here when it arrives, for the tab session, and the bar reads it back.
import type { Pod } from '../../gen/ts/protocol';

const KEY = 'jm.podNames';
const seen = new Map<string, string>();

function stored(): Record<string, string> {
  try {
    const raw = sessionStorage.getItem(KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : {};
    return typeof parsed === 'object' && parsed !== null ? (parsed as Record<string, string>) : {};
  } catch { return {}; }
}

export function rememberPod(pod: Pod): void {
  seen.set(pod.id, pod.name);
  try { sessionStorage.setItem(KEY, JSON.stringify({ ...stored(), [pod.id]: pod.name })); } catch { /* storage off: the name lasts until reload */ }
}

/** The bar's heading for a pod: its name once known, otherwise the id (and nothing better is known yet). */
export function podHeading(id: string): string {
  const name = seen.get(id) ?? stored()[id];
  return typeof name === 'string' && name !== '' ? name : `Pod ${id}`;
}
