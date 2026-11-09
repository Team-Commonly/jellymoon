#!/usr/bin/env bun
// Tests the pure pieces of `land.mjs --batch`: the argument shape, the plan, the ref shape and the per-merge step check. Own step in `make test-ts`.
import { isLandRef, landRef } from './land-decide.mjs';
import { batchProof, checkBatchStep, parseBatch, planBatch, prefixes } from './land-batch.mjs';

const [H, H2] = ['a'.repeat(40), 'c'.repeat(40)];
const ok = (n) => ({ n, ok: true });
const refused = (a) => parseBatch(a).ok;
const lands = (v) => { try { landRef(v); return 'ok'; } catch { return 'throws'; } };
const cases = [
  ['batch args: --batch with PR numbers, in the given order, and --dry-run', JSON.stringify(parseBatch(['--batch', '7', '5', '9', '--dry-run'])), '{"ok":true,"nums":["7","5","9"]}'],
  ['batch args: no --batch is not a batch', parseBatch(['5']), null],
  ['batch args: one PR, a repeat, a non-number, bootstrap, preflight-only and more than eight are refused', [['--batch', '5'], ['--batch', '5', '5'], ['--batch', '5', 'x'], ['--batch', '5', '6', '--bootstrap'], ['--batch', '5', '6', '--preflight-only'], ['--batch', ...[1, 2, 3, 4, 5, 6, 7, 8, 9].map(String)]].map(refused).join(), 'false,false,false,false,false,false'],
  ['batch plan: every PR that passed its gate stays, in order', JSON.stringify(planBatch([ok('5'), ok('6'), ok('7')])), '{"keep":["5","6","7"],"dropped":[],"mode":"batch"}'],
  ['batch plan: a gated-out PR is dropped with its reason and the rest go on', JSON.stringify(planBatch([ok('5'), { n: '6', ok: false, reason: 'no verifier PASS' }, ok('7')])), '{"keep":["5","7"],"dropped":[{"n":"6","reason":"no verifier PASS"}],"mode":"batch"}'],
  ['batch plan: one survivor goes the single path, none lands nothing', [planBatch([ok('5'), { n: '6', ok: false, reason: 'x' }]).mode, planBatch([{ n: '5', ok: false, reason: 'x' }]).mode].join(), 'single,none'],
  ['batch ref: land/<digits>(-<digits>)* only', [landRef('5-6-7'), landRef(21), landRef('007-8')].join(), 'refs/heads/land/5-6-7,refs/heads/land/21,refs/heads/land/7-8'],
  ['batch ref: a trailing or doubled dash, a path or a branch name throws', ['5-', '-5', '5--6', '5/6', 'main', '5-x', '5 6'].map(lands).join(), 'throws,throws,throws,throws,throws,throws,throws'],
  ['batch ref: the delete guard accepts exactly that shape', `${['refs/heads/land/5', 'refs/heads/land/5-6-7'].map(isLandRef).join()}|${['refs/heads/land/', 'refs/heads/land/5-', 'refs/heads/land/5/6', 'refs/heads/main', 'refs/heads/feat', 'refs/heads/land/a-1'].map(isLandRef).join()}`, 'true,true|false,false,false,false,false,false'],
  ['batch step: main at the expected tip and the head unmoved goes on', checkBatchStep({ expect: H, tip: H, head: H2, headNow: H2 }).ok, true],
  ['batch step: main moved mid-batch stops, naming the move', checkBatchStep({ expect: H, tip: H2, head: H, headNow: H }).reason.includes('main moved'), true],
  ['batch step: a PR head that moved stops', checkBatchStep({ expect: H, tip: H, head: H, headNow: H2 }).ok, false],
  ['batch prefixes: one per chain step, id = the PR numbers so far, with that step\'s parent, head and tree', JSON.stringify(prefixes([{ n: '5', parent: 'm', head: 'h5', tree: 't5' }, { n: '6', parent: 'l1', head: 'h6', tree: 't6' }])), '[{"id":"5","parent":"m","head":"h5","tree":"t5"},{"id":"5-6","parent":"l1","head":"h6","tree":"t6"}]'],
  ['batch proof: every prefix proved is a proof', batchProof([{ id: '5', ok: true }, { id: '5-6', ok: true }]).ok, true],
  ['batch proof: one red or unproved prefix, a middle one included, is no proof and names it', JSON.stringify(batchProof([{ id: '5', ok: true }, { id: '5-6', ok: false, reason: 'x' }, { id: '5-6-7', ok: true }])), '{"ok":false,"reason":"land/5-6: x"}'],
];
const bad = cases.filter(([, got, want]) => got !== want).map(([n]) => n);
if (bad.length) { console.error(`check-land-batch: ${bad.length} of ${cases.length} wrong\n  ${bad.join('\n  ')}`); process.exit(1); }
console.log(`check-land-batch: ${cases.length} cases ok`);
