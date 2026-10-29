#!/usr/bin/env bun
// Validates every fixture in schema/fixtures/<Def>/ against the generated bundle (the build plan L0 done-test).
// valid-*.json must pass and invalid-*.json must fail. An Envelope fixture's data is also checked
// against the definition its event type names. The Go side of the same check is
// daemon/internal/protocol/fixtures_test.go.
// A def dir may hold expect.json: { "invalid-x.json": { "keyword": "maxProperties", "path": "/needs/to" } }.
// Then every ajv error on the fixture must have that keyword (and instance path, when given), so it fails
// for its named rule only. A def with invalid fixtures needs an expect.json covering all of them, unless it is
// in LEGACY_UNASSERTED (defs that predate this check; the list only shrinks).
import fs from 'node:fs';
import path from 'node:path';
import Ajv2020 from 'ajv/dist/2020.js';

const ROOT = path.resolve(import.meta.dirname, '..');
const bundle = JSON.parse(fs.readFileSync(path.join(ROOT, 'gen/ts/schemas.json'), 'utf8'));
const variants = JSON.parse(fs.readFileSync(path.join(ROOT, 'schema/events.json'), 'utf8'))['x-variants'];
const ajv = new Ajv2020({ allErrors: true, strict: true });
ajv.addSchema(bundle);
const validator = (def) => ajv.getSchema(`${bundle.$id}#/$defs/${def}`) || fail(`no def ${def}`);
const defName = (ref) => ref.split('/').pop();

function fail(msg) { console.error(msg); process.exit(1); }

const LEGACY_UNASSERTED = new Set(['ClaimReleased', 'Envelope', 'ReadInput', 'RegisterComputerRequest',
  'RunStopInput', 'RunStopOutput', 'Task', 'TaskClaimInput', 'TaskReleaseInput', 'TaskUpdateInput']);
const LEGACY_PIN = 10; // lower this when a def leaves LEGACY_UNASSERTED; it may never grow
if (LEGACY_UNASSERTED.size !== LEGACY_PIN) fail(`LEGACY_UNASSERTED has ${LEGACY_UNASSERTED.size} defs, pinned at ${LEGACY_PIN}: it only shrinks, so lower the pin when you remove one`);
const dir = path.join(ROOT, 'schema/fixtures');
let checked = 0;
const problems = [];
for (const def of fs.readdirSync(dir).sort()) {
  const validate = validator(def);
  const expectPath = path.join(dir, def, 'expect.json');
  const expects = fs.existsSync(expectPath) ? JSON.parse(fs.readFileSync(expectPath, 'utf8')) : null;
  const files = fs.readdirSync(path.join(dir, def)).filter((f) => f !== 'expect.json').sort();
  for (const k of Object.keys(expects || {})) if (!files.includes(k)) problems.push(`${def}/expect.json: no fixture ${k}`);
  if (!expects && !LEGACY_UNASSERTED.has(def) && files.some((f) => f.startsWith('invalid-'))) problems.push(`${def}: invalid fixtures need an expect.json`);
  for (const file of files) {
    const value = JSON.parse(fs.readFileSync(path.join(dir, def, file), 'utf8'));
    let ok = validate(value);
    let errs = validate.errors;
    if (ok && def === 'Envelope') {
      const dataDef = validator(defName(variants[value.type].data));
      ok = dataDef(value.data);
      errs = dataDef.errors;
    }
    const wantValid = file.startsWith('valid-');
    if (!wantValid && !file.startsWith('invalid-')) problems.push(`${def}/${file}: name must start with valid- or invalid-`);
    else if (ok !== wantValid) problems.push(`${def}/${file}: expected ${wantValid ? 'valid' : 'invalid'}${errs ? ` — ${ajv.errorsText(errs)}` : ''}`);
    const want = expects && expects[file];
    if (!wantValid && expects && !want) problems.push(`${def}/${file}: no entry in expect.json`);
    else if (!wantValid && want && !ok && !(errs || []).every((e) => e.keyword === want.keyword && (want.path === undefined || e.instancePath === want.path))) {
      problems.push(`${def}/${file}: expected ${want.keyword}${want.path === undefined ? '' : ` at ${want.path || '(root)'}`}, got ${ajv.errorsText(errs)}`);
    }
    checked++;
  }
}
if (problems.length) fail(`check-fixtures: ${problems.length} of ${checked} wrong\n  ${problems.join('\n  ')}`);
console.log(`check-fixtures: ${checked} fixtures ok`);
