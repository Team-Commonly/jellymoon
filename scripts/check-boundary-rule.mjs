// Pins the ee-stays-in-ee rule (ADR 0004): depcruise runs on the tracked pair in boundary-fixture/ (a module outside ee/
// importing one inside it) and must report exactly that violation. Deleting or weakening the rule fails preflight.
import { execFileSync } from 'node:child_process';

let out;
try {
  out = execFileSync('../node_modules/.bin/depcruise', ['.', '--config', '../.dependency-cruiser.cjs', '-T', 'json'], { cwd: 'boundary-fixture', encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
} catch (e) { out = e.stdout; } // depcruise exits non-zero on a violation, which is what we expect
const found = JSON.parse(out).summary.violations.map((v) => `${v.rule.name}: ${v.from} -> ${v.to}`);
const want = ['ee-stays-in-ee: core.mjs -> ee/leaf.mjs'];
if (JSON.stringify(found) !== JSON.stringify(want)) {
  console.error(`check-boundary-rule: expected exactly ${JSON.stringify(want)}, got ${JSON.stringify(found)}`);
  process.exit(1);
}
console.log('check-boundary-rule: the fixture import of ee/ is refused by ee-stays-in-ee');
