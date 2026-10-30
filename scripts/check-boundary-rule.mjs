// Pins the ee-stays-in-ee rule (ADR 0004). Three things must hold: the rule is an error with an unexempted source (the
// fixture lives outside every real source dir, so a widened pathNot would still pass the run below); depcruise run on the
// tracked pair in boundary-fixture/ (a module outside ee/ importing one inside it) exits non-zero; and it reports exactly
// that violation. Deleting, downgrading or exempting the rule fails preflight.
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';

const fail = (msg) => { console.error(`check-boundary-rule: ${msg}`); process.exit(1); };

const rule = createRequire(import.meta.url)('../.dependency-cruiser.cjs').forbidden.find((r) => r.name === 'ee-stays-in-ee');
if (!rule) fail('the ee-stays-in-ee rule is missing');
if (rule.severity !== 'error') fail(`the rule severity is ${rule.severity}, want error`);
if (rule.from.pathNot !== '^ee/' || rule.from.path !== undefined) fail('the rule source must be everything outside ee/, with no other exemption');
if (rule.to.path !== '^ee/') fail('the rule must refuse every import of ee/');

const dep = (type) => spawnSync('../node_modules/.bin/depcruise', ['.', '--config', '../.dependency-cruiser.cjs', '-T', type], { cwd: 'boundary-fixture', encoding: 'utf8' });
if (dep('err').status === 0) fail('depcruise exited 0 on a core file importing ee/');
const r = dep('json');
let found;
try { found = JSON.parse(r.stdout).summary.violations.map((v) => `${v.rule.name}: ${v.from} -> ${v.to}`); } catch { fail('depcruise printed no JSON report'); }
const want = ['ee-stays-in-ee: core.mjs -> ee/leaf.mjs'];
if (JSON.stringify(found) !== JSON.stringify(want)) fail(`expected exactly ${JSON.stringify(want)}, got ${JSON.stringify(found)}`);
console.log('check-boundary-rule: the fixture import of ee/ is refused by ee-stays-in-ee');
