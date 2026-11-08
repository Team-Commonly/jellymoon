// Writes a fresh PGlite data directory as one tar: `bun server/src/pglite-template.ts <file>`. A local run starts from it
// (PGLITE_TEMPLATE) instead of running initdb for every server, which is most of PGlite's start-up cost. Not used in production.
import { renameSync, writeFileSync } from 'node:fs';
import { PGlite } from '@electric-sql/pglite';

const out = process.argv[2];
if (out === undefined) {
  console.error('usage: pglite-template.ts <file>');
  process.exit(2);
}
const pg = new PGlite();
await pg.waitReady;
const tar = (await pg.dumpDataDir('none')) as Blob;
// Written beside the target and renamed, so a reader never sees half a template.
writeFileSync(`${out}.part`, new Uint8Array(await tar.arrayBuffer()));
renameSync(`${out}.part`, out);
process.exit(0);
