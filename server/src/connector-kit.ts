// What the connector routes (connectors.ts) and the agent's call (connector-call.ts) share (ADR 0009).
import { destOf } from './egress';
import type { Dest, Resolve } from './egress';
import type { Hub } from './hub';
import { fail } from './kit';
import type { Call } from './kit';
import type { ConnRow } from './store';
import type { Adapter } from './upstream';
import type { Vault } from './vault';

/**
 * What every connector handler needs: the hub, the vault (undefined without JM_VAULT_KEY), whether this is ENV=local, and the
 * resolver. `adapter` replaces the pin's protocol adapter in tests; the server never sets it.
 */
export interface Kit { hub: Hub; vault: Vault | undefined; local: boolean; resolve: Resolve; adapter?: Adapter }

/** The checked, pinned destination of a stored connection, checked again for this one request; or why nothing may be sent. */
export const destFor = (kit: Kit, c: Pick<ConnRow, 'url'>): Promise<Dest | string> => destOf(c.url, kit.local, kit.resolve);

/** The additional data a connection's envelope is bound to: its pod and its own id. */
export const aadOf = (c: Pick<ConnRow, 'pod' | 'id'>): string => `${c.pod}/${c.id}`;

export const noVault = (k: Call): Response => fail(k.c, 'vault_unconfigured', 'This server has no vault key (JM_VAULT_KEY); it cannot hold a connection credential.');

/** The run's agent, or undefined for a person. */
export const agentOf = (k: Call): string | undefined => (k.caller.kind === 'run' ? k.pod.state.runs[k.caller.runId]?.agent.id : undefined);
