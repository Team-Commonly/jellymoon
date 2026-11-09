#!/usr/bin/env bun
// End-to-end cases for `land.mjs --batch`: main() runs against the scripted fake in land-fake.mjs (see check-land-e2e.mjs for the single path).
import { createHash } from 'node:crypto';
import { MAIN, HEAD, OTHER, REPO, HDR, NOTE, fail, HEADS, ALL, land } from './land-fake.mjs';

const cases = [];
const t = async (name, over, f) => { cases.push([name, !!f(await land(over)), true]); };
const no = (r, re) => !r.merge && re.test(r.error ?? '');
const argv = (r, cmd, sub) => r.w.calls.filter((c) => c.cmd === cmd && [cmd, ...c.args].join(' ').includes(sub)).map((c) => c.args.join(' '));
const RULES = '{"type":"pull_request","parameters":{}}\n{"type":"required_status_checks","parameters":{"required_status_checks":[{"context":"ci","integration_id":15368}]}}';
const crun = (o = {}) => ({ name: 'ci', status: 'completed', conclusion: 'success', app: 'github-actions', suite: 1, ...o }); // suite 1 is land's own push to the batch ref, 9 a pull_request suite
const pending = crun({ status: 'in_progress', conclusion: null });
const pushes = (r) => r.w.calls.filter((c) => c.cmd === 'git' && c.args[0] === 'push').map((c) => c.args);
const sameArgs = (a, b) => JSON.stringify(a) === JSON.stringify(b);
// LBATCH: `--batch 5 6 7` builds a chain of land commits (main tip + head 5, then + head 6, ...), pushes only the last to land/5-6-7, waits for CI on it, and
// merges each PR in order through the same gh call; red or no proof lands nothing from it and each PR goes the single path. The fake is a small git/GitHub:
// trees follow the commits (a tree is a function of its parent's tree and the head), so a merge onto the merge commits leaves main at the last land tree.
const sha1 = (...a) => createHash('sha1').update(a.join('|')).digest('hex');
const BH = { 5: HEAD, 6: '6'.repeat(40), 7: '7'.repeat(40) };
const MAINTREE = sha1('main tree');
const treeFor = (parentTree, head) => sha1('tree', parentTree, head);
const bledger = (skip) => HDR + [5, 6, 7].filter((n) => n !== skip).map((n) => `${n}\t${BH[n]}\tPASS\topus-verifier\t2026-10-29T10:00:00Z\tn\n`).join('');
const bmsgs = [5, 6, 7].map((n) => ({ ...NOTE, content: `jellymoon#${n} ${BH[n]}\nok\nGATE: PASS` }));
HEADS.push(BH[6], BH[7]);
const bci = (o = {}) => {
  const st = (w) => (w.b ??= { trees: { [MAIN]: MAINTREE }, tip: MAIN, checkout: MAIN, M: {}, commits: [], pushed: new Set(), refOf: {}, merged: [] });
  const fields = (n, a, w) => { const pr = { url: `https://github.com/${REPO}/pull/${n}`, number: Number(n), state: 'OPEN', baseRefName: o.retarget === n && st(w).pushed.size ? 'dev' : 'main', headRefName: `feat${n}`, headRefOid: o.moveHead === n && st(w).merged.length ? OTHER : BH[n], createdAt: '2026-10-29T09:00:00Z', isCrossRepository: false, isDraft: false, author: { login: 'alice' }, ...o.pr?.[n] }; return JSON.stringify(Object.fromEntries(a[a.indexOf('--json') + 1].split(',').map((k) => [k, pr[k]]))); };
  const sha = (a) => a.find((x) => /commits\/[0-9a-f]{40}/.test(x)).match(/commits\/([0-9a-f]{40})/)[1];
  return { ledger: o.ledger ?? bledger(), msgs: bmsgs, on: [
    [/^gh pr view (\d+) .*--json mergeCommit/, (a, w) => JSON.stringify({ mergeCommit: { oid: st(w).M[a[3]] } })],
    [/^gh pr view (\d+) /, (a, w) => fields(a[3], a, w)],
    [/^gh pr merge (\d+) /, (a, w) => { const b = st(w); const m = sha1('M', b.tip, a.at(-1)); b.trees[m] = o.mismatch === a[3] ? sha1('wrong tree') : treeFor(b.trees[b.tip], a.at(-1)); b.M[a[3]] = m; b.tip = m; b.merged.push(a[3]); if (o.moveAfter === b.merged.length) { b.tip = OTHER; b.trees[OTHER] = sha1('elsewhere'); } return ''; }],
    [/^git rev-parse HEAD$/, (a, w) => st(w).checkout],
    [/^git (-c \S+ )?merge --ff-only /, (a, w) => { st(w).checkout = a.at(-1); return ''; }],
    [/^git rev-parse FETCH_HEAD/, (a, w) => (w.fetched === 'refs/heads/main' ? st(w).tip : w.fetched.startsWith('pull/') ? BH[w.fetched.split('/')[1]] : w.fetched)],
    [/^git rev-parse (\w+)\^\{tree\}/, (a, w) => { const t = st(w).trees[a[2].slice(0, 40)]; if (!t) throw fail; return t; }],
    [/^git merge-tree --write-tree/, (a, w) => { if ((o.conflict || []).includes(a[4]) && a[3] !== MAIN) throw fail; return `${treeFor(st(w).trees[a[3]], a[4])}\n`; }],
    [/^git (-c \S+ )?commit-tree /, (a, w) => { const b = st(w); const tree = a[a.indexOf('--no-gpg-sign') + 1]; const parents = a.flatMap((x, i) => (a[i - 1] === '-p' ? [x] : [])); const s = sha1('L', tree, ...parents); b.trees[s] = tree; b.commits.push({ sha: s, parents }); return s; }],
    [/^git ls-remote /, ''], [/^git show -s --format=%ct /, '1700000000\n1700000100'],
    [/^git push /, (a, w) => { const b = st(w); if (!a.includes('--delete')) { const [s, ref] = a.at(-1).split(':'); b.pushed.add(s); b.refOf[s] = ref; } return ''; }],
    [/\/rules\/branches\/main/, RULES], [/\/branches\/main\/protection\/required_status_checks/, () => { throw fail; }],
    [/actions\/runs\?head_sha=\w+&event=push&branch=land\/[\d-]+&/, JSON.stringify({ total: 1, count: 1, ids: [1] })],
    [/\/git\/commits\/[0-9a-f]{40}/, (a, w) => { const s = sha(a); if (!st(w).pushed.has(s) && !o.prior) throw fail; return st(w).trees[s]; }],
    [/commits\/[0-9a-f]{40}\/check-runs/, (a, w) => {
      const b = st(w); const s = sha(a);
      if (!b.commits.some((c) => c.sha === s)) return '{"total":1,"runs":[{"name":"ci","conclusion":"success"}]}'; // a PR head's own checks
      const runs = !b.pushed.has(s) ? (o.prior ?? []) : o.refRuns?.[b.refOf[s]] ?? (/land\/\d+-/.test(b.refOf[s]) ? (o.batchRuns ?? [crun()]) : [crun()]);
      return JSON.stringify({ total: runs.length, runs });
    }],
  ] };
};
const bland = (argv, o = {}) => ({ ...bci(o), argv, ...o.over });
const merges = (r) => r.w.calls.filter((c) => c.cmd === 'gh' && c.args[1] === 'merge').map((c) => c.args[2]);
const refsOf = (r, del) => pushes(r).filter((a) => a.includes('--delete') === del).map((a) => (del ? a.at(-1) : a.at(-1).split(':')[1]));
const chainOf = (r) => [...new Map((r.w.b?.commits ?? []).map((c) => [c.sha, c])).values()];
const landed = (r) => r.ledgerNow.split('\n').filter((l) => l.includes('\tLANDED\t')).map((l) => l.split('\t')[0]);
const delIdx = (r, ref) => r.w.calls.findIndex((c) => c.cmd === 'git' && c.args.includes('--delete') && c.args.at(-1) === ref);
const B567 = ['--batch', '5', '6', '7'];
await t('batch: three green PRs push each prefix to its own ref, then three merges in the given order, main ending at the last land tree', bland(B567), (r) => {
  const [l1, l2, l3] = chainOf(r); const b = r.w.b; if (!l3) return false;
  return sameArgs(refsOf(r, false), ['refs/heads/land/5', 'refs/heads/land/5-6', 'refs/heads/land/5-6-7']) && sameArgs(pushes(r)[2], ['push', '--quiet', '--force-with-lease=refs/heads/land/5-6-7:', 'origin', `${l3.sha}:refs/heads/land/5-6-7`])
    && sameArgs(merges(r), ['5', '6', '7']) && sameArgs(landed(r), ['5', '6', '7']) && r.w.spawns.length === 0 && r.error === null
    && sameArgs(l1.parents, [MAIN, BH[5]]) && sameArgs(l2.parents, [l1.sha, BH[6]]) && sameArgs(l3.parents, [l2.sha, BH[7]]) && b.trees[b.tip] === b.trees[l3.sha];
});
await t('batch: every batch ref is deleted after the last merge', bland(B567), (r) => sameArgs([...refsOf(r, true)].sort(), ['refs/heads/land/5', 'refs/heads/land/5-6', 'refs/heads/land/5-6-7']) && delIdx(r, 'refs/heads/land/5-6-7') > r.w.calls.findLastIndex((c) => c.cmd === 'gh' && c.args[1] === 'merge'));
await t('batch: every merge is the single path\'s call, pinned to its own head', bland(B567), (r) => merges(r).length === 3 && r.w.calls.filter((c) => c.cmd === 'gh' && c.args[1] === 'merge').every((c) => sameArgs(c.args, ['pr', 'merge', c.args[2], '--repo', REPO, '--merge', '--match-head-commit', BH[c.args[2]]])));
await t('batch: the CI poll reads only push suites of the batch refs', bland(B567), (r) => argv(r, 'gh', 'actions/runs?head_sha=').length >= 3 && argv(r, 'gh', 'actions/runs?head_sha=').every((a) => /&event=push&branch=land\/\d+(-\d+)*&/.test(a)));
await t('batch: a PR that fails its gate (no ledger PASS) is dropped with its reason; the other two are a batch of 2', bland(B567, { ledger: bledger(6) }), (r) => sameArgs(merges(r), ['5', '7']) && sameArgs(refsOf(r, false), ['refs/heads/land/5', 'refs/heads/land/5-7']) && r.said.includes('#6 dropped from the batch') && r.said.includes('no verifier PASS for #6') && sameArgs(chainOf(r).map((c) => c.parents[1]), [BH[5], BH[7]]));
await t('batch: a PR with no pod verdict is dropped too', { ...bland(B567), msgs: bmsgs.filter((m) => !m.content.startsWith('jellymoon#7')) }, (r) => sameArgs(merges(r), ['5', '6']) && r.said.includes('#7 dropped from the batch'));
await t('batch: a draft PR is dropped before anything is built', bland(B567, { pr: { 6: { isDraft: true } } }), (r) => sameArgs(merges(r), ['5', '7']) && r.said.includes('#6 dropped from the batch') && r.said.includes('is a draft'));
await t('batch: a PR that conflicts in the chain is dropped and the next one builds on the commit before it', bland(B567, { conflict: [BH[6]] }), (r) => { const [l1, l2] = chainOf(r); return sameArgs(merges(r), ['5', '7']) && r.said.includes('#6 dropped from the batch') && sameArgs(l2.parents, [l1.sha, BH[7]]) && refsOf(r, false)[1] === 'refs/heads/land/5-7'; });
await t('batch: one PR left after the drops goes the single path (land/<n>)', bland(['--batch', '5', '6'], { ledger: bledger(6) }), (r) => sameArgs(merges(r), ['5']) && sameArgs(refsOf(r, false), ['refs/heads/land/5']) && r.said.includes('#6 dropped from the batch'));
await t('batch: no PR passing its gate lands nothing', bland(['--batch', '5', '6'], { ledger: HDR }), (r) => merges(r).length === 0 && pushes(r).length === 0 && /no PR in the batch passed/.test(r.error ?? ''));
await t('batch: a red batch lands nothing from it, says so, deletes its ref, and then each PR goes the single path in order', bland(B567, { batchRuns: [crun({ conclusion: 'failure' })] }), (r) => {
  const first = r.w.calls.findIndex((c) => c.cmd === 'gh' && c.args[1] === 'merge');
  return r.said.includes('batch red; landing one at a time') && sameArgs(merges(r), ['5', '6', '7']) && sameArgs(refsOf(r, false), ['refs/heads/land/5', 'refs/heads/land/5-6', 'refs/heads/land/5-6-7', 'refs/heads/land/6', 'refs/heads/land/7'])
    && delIdx(r, 'refs/heads/land/5-6-7') < first && sameArgs(landed(r), ['5', '6', '7']) && r.error === null;
});
await t('batch: no CI proof (the cap) is red too: nothing from the batch lands, singles follow', bland(B567, { batchRuns: [] }), (r) => r.said.includes('batch red; landing one at a time') && sameArgs(merges(r), ['5', '6', '7']) && r.w.sleeps.length === 90);
await t('batch: green runs of a pull_request suite on the batch SHA (a fork PR) never count; the batch is pushed anyway and goes red', bland(B567, { prior: [crun({ suite: 9 })], batchRuns: [crun({ suite: 9 })] }), (r) => refsOf(r, false)[2] === 'refs/heads/land/5-6-7' && r.said.includes('batch red; landing one at a time') && r.said.includes('has no run on the land commit yet'));
await t('batch: main moving after the first merge stops the batch there; the other two go the single path', bland(B567, { moveAfter: 1 }), (r) => sameArgs(merges(r), ['5', '6', '7']) && r.said.includes('main moved') && sameArgs(refsOf(r, false), ['refs/heads/land/5', 'refs/heads/land/5-6', 'refs/heads/land/5-6-7', 'refs/heads/land/6', 'refs/heads/land/7']) && sameArgs(landed(r), ['5', '6', '7']));
await t('batch: a PR head that moved before its merge stops the batch there; that PR is refused by the single path (no verdict for the new head) and the next one lands', bland(B567, { moveHead: '6' }), (r) => sameArgs(merges(r), ['5', '7']) && r.said.includes('differs from the local fetch') && !landed(r).includes('6') && landed(r).includes('7'));
await t('batch: a PR dropped at its gate while the rest merge makes the run exit non-zero', bland(B567, { ledger: bledger(6) }), (r) => sameArgs(merges(r), ['5', '7']) && /^1 of 3 PRs did not land/.test(r.error ?? ''));
await t('batch: a PR dropped for a chain conflict while the rest merge makes the run exit non-zero', bland(B567, { conflict: [BH[6]] }), (r) => sameArgs(merges(r), ['5', '7']) && /^1 of 3 PRs did not land/.test(r.error ?? ''));
await t('batch: a gate drop plus a single-path refusal counts both', bland(B567, { ledger: bledger(6), batchRuns: [crun({ conclusion: 'failure' })], moveHead: '7' }), (r) => /^2 of 3 PRs did not land/.test(r.error ?? '') && !landed(r).includes('6'));
await t('batch: every PR landing exits clean', bland(B567), (r) => r.error === null);
const noteOf = (r, n) => r.ledgerNow.split('\n').find((l) => l.startsWith(`${n}\t`) && l.includes('\tLANDED\t')) ?? '';
await t('batch: each LANDED note quotes its own prefix\'s proof (its tree), not the final prefix\'s', bland(B567), (r) => { const c = chainOf(r); const tr = (i) => r.w.b.trees[c[i].sha].slice(0, 8); return c.length === 3 && [5, 6, 7].every((n, i) => noteOf(r, n).includes(`batch CI proved merge tree ${tr(i)}`)) && !noteOf(r, 5).includes(tr(2)); });
await t('batch: "landed as" prints only after the tree check passes; a mismatch says TREE-MISMATCH and never "landed as"', bland(B567, { mismatch: '6' }), (r) => sameArgs(merges(r), ['5', '6']) && r.said.includes('#5 landed as') && !r.said.includes('#6 landed as') && /not the tested batch tree/.test(r.error ?? '') && noteOf(r, 6).includes('TREE-MISMATCH'));
const viaBatch = (r) => r.said.includes('(batch)');
await t('batch: a PR retargeted to another base during the CI wait is not merged by the batch (the single path\'s base check, on the fresh read); the rest go on', bland(B567, { retarget: '6' }), (r) => !merges(r).includes('6') && r.said.includes('is based on dev, not main') && r.said.includes('#5 landed as') && landed(r).includes('7'));
await t('batch: a red middle prefix with a green final proves nothing: no batch merge, the single path for each PR', bland(B567, { refRuns: { 'refs/heads/land/5-6': [crun({ conclusion: 'failure' })] } }), (r) => r.said.includes('batch red; landing one at a time') && !viaBatch(r) && sameArgs(merges(r), ['5', '6', '7']) && r.w.calls.findIndex((c) => c.cmd === 'gh' && c.args[1] === 'merge') > delIdx(r, 'refs/heads/land/5-6-7'));
await t('batch: a middle prefix with no proof (the cap) is no proof: no batch merge, the single path for each PR', bland(B567, { refRuns: { 'refs/heads/land/5-6': [] } }), (r) => r.said.includes('batch red; landing one at a time') && !viaBatch(r) && sameArgs(merges(r), ['5', '6', '7']));
await t('batch: every prefix of the chain is pushed to its own ref and deleted again, in one CI cycle (the waits run side by side)', bland(B567), (r) => sameArgs(refsOf(r, false), ['refs/heads/land/5', 'refs/heads/land/5-6', 'refs/heads/land/5-6-7']) && sameArgs([...refsOf(r, true)].sort(), ['refs/heads/land/5', 'refs/heads/land/5-6', 'refs/heads/land/5-6-7']) && r.w.sleeps.length === 0 && r.said.split('\n').filter((l) => l.includes('(batch)')).length === 3);
await t('batch: --dry-run runs the gates, builds the chain, pushes and merges nothing, and prints the plan', bland([...B567, '--dry-run']), (r) => pushes(r).length === 0 && merges(r).length === 0 && r.said.includes('dry run: would merge') && r.said.includes('land/5-6-7') && chainOf(r).length >= 3 && r.ledgerNow === r.ledgerWas);
await t('batch: an interrupt while waiting deletes every batch ref', { ...bland(B567, { batchRuns: [pending] }), onSleep: (w) => { if (w.sleeps.length === 3) process.emit('exit', 130); } }, (r) => sameArgs(refsOf(r, true).slice(0, 3).sort(), ['refs/heads/land/5', 'refs/heads/land/5-6', 'refs/heads/land/5-6-7']));
await t('batch: --batch with one PR, a repeat, or --bootstrap is a usage refusal', { argv: ['--batch', '5'] }, (r) => no(r, /two to eight/));
await t('batch: --batch with --preflight-only is refused', { argv: ['--batch', '5', '6', '--preflight-only'] }, (r) => no(r, /PR numbers and --dry-run only/));
cases.push(['batch: across every batch scenario, every git push is a create-with-lease or a delete of a land/<digits>(-<digits>)* ref', ALL.some((c) => c.cmd === 'git' && c.args[0] === 'push') && ALL.filter((c) => c.cmd === 'git' && c.args[0] === 'push').every((c) => { const a = c.args; const ref = a.at(-1).replace(/^[0-9a-f]{40}:/, ''); return /^refs\/heads\/land\/\d+(-\d+)*$/.test(ref) && (a.includes('--delete') ? a.length === 5 : a.length === 5 && a[2].startsWith(`--force-with-lease=${ref}:`) && a[3] === 'origin'); }), true]);


const bad = cases.filter(([, got, want]) => got !== want);
for (const [n] of bad) console.error(`  ${n}`);
if (bad.length) { console.error(`check-land-batch-e2e: ${bad.length} of ${cases.length} wrong`); process.exit(1); }
console.log(`check-land-batch-e2e: ${cases.length} cases ok`);
