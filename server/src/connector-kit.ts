// What the connector routes (connectors.ts) and the agent's call (connector-call.ts) share (ADR 0009).
import type { Hub } from './hub';
import { fail } from './kit';
import type { Call } from './kit';
import type { ConnRow } from './store';
import type { Vault } from './vault';

/** What every connector handler needs: the hub, the vault (undefined without JM_VAULT_KEY) and whether this is ENV=local. */
export interface Kit { hub: Hub; vault: Vault | undefined; local: boolean }

/** The additional data a connection's envelope is bound to: its pod and its own id. */
export const aadOf = (c: Pick<ConnRow, 'pod' | 'id'>): string => `${c.pod}/${c.id}`;

export const noVault = (k: Call): Response => fail(k.c, 'vault_unconfigured', 'This server has no vault key (JM_VAULT_KEY); it cannot hold a connection credential.');

/** The run's agent, or undefined for a person. */
export const agentOf = (k: Call): string | undefined => (k.caller.kind === 'run' ? k.pod.state.runs[k.caller.runId]?.agent.id : undefined);
