// Which adapter speaks to an upstream. One adapter file per protocol, each with list and call; a new protocol (OpenAPI,
// plain REST) is a new file and one line in ADAPTERS (ADR 0009). Where an upstream may be is egress.ts.
import type { Pin } from './catalogue';
import * as mcpHttp from './upstream-mcp';

export type Adapter = typeof mcpHttp;
const ADAPTERS: Record<Pin['protocol'], Adapter> = { 'mcp-http': mcpHttp };
export const adapterOf = (pin: Pin, override?: Adapter): Adapter => override ?? ADAPTERS[pin.protocol];
