// `land.mjs --batch <a> <b> ...` (docs/development/landing.md, "Batch landing"). Each PR passes the single path's gates first; the survivors become a chain of
// land commits (L1 = main + a, L2 = L1 + b, ...), each prefix is pushed to its own ref (land/<a>, land/<a>-<b>, ...) and proved by CI, and only when every prefix is
// green do the PRs merge in order. Red, a prefix with no proof, or anything moved: the rest land one at a time. `c` is land.mjs's own functions (this file reads and decides nothing itself); the pure pieces are in land-decide.mjs.
import { landAction, checkHead } from './land-decide.mjs';

// The pure pieces (tested by check-land-batch.mjs).
// `land.mjs --batch <a> <b> ...`: two to eight distinct PR numbers, in landing order, and --dry-run; nothing else. null when it is not a batch.
export function parseBatch(args) {
  if (!args.includes('--batch')) return null;
  const nums = args.filter((a) => /^\d+$/.test(a));
  if (args.some((a) => !/^\d+$/.test(a) && !['--batch', '--dry-run'].includes(a))) return { ok: false, reason: '--batch takes PR numbers and --dry-run only' };
  if (nums.length < 2 || nums.length > 8 || new Set(nums.map(Number)).size !== nums.length) return { ok: false, reason: '--batch needs two to eight different PR numbers' };
  return { ok: true, nums };
}
// entries: [{ n, ok, reason }] in landing order, each the result of that PR's own gates. A PR that failed is dropped with its reason; fewer than two survivors is no batch.
export function planBatch(entries) {
  const keep = entries.filter((e) => e.ok).map((e) => e.n);
  return { keep, dropped: entries.filter((e) => !e.ok).map((e) => ({ n: e.n, reason: e.reason })), mode: keep.length >= 2 ? 'batch' : keep.length ? 'single' : 'none' };
}
// Before merging PR k of a batch: main must be exactly where the previous merge left it (`expect`) and the PR head the one that was tested.
export const checkBatchStep = ({ expect, tip, head, headNow }) => (tip !== expect ? { ok: false, reason: `main moved from ${expect.slice(0, 8)} to ${tip.slice(0, 8)} during the batch` }
  : head.toLowerCase() !== headNow.toLowerCase() ? { ok: false, reason: `the head moved from ${head.slice(0, 8)} to ${headNow.slice(0, 8)}` } : { ok: true });

// GitHub merges one PR at a time, so main passes through every prefix L1, L2, ... of the chain; each is proved on its own ref (land/<a>, land/<a>-<b>, ...).
export const prefixes = (chain) => chain.map((p, i) => ({ id: chain.slice(0, i + 1).map((x) => x.n).join('-'), parent: p.parent, head: p.head, tree: p.tree }));
// results: ciLand's answer for each prefix, in order. The batch is proved only when every one is; the first that is not is named.
export const batchProof = (results) => { const bad = results.find((r) => !r.ok); return bad ? { ok: false, reason: `land/${bad.id}: ${bad.reason}` } : { ok: true }; };

export async function batch(c, nums, hold) {
  const { out, say, refuse, Refusal, oneLine, why } = c;
  hold(c.lock());
  const mainTip = c.fetchRef(c.MAIN_REF);
  c.checkoutGate(mainTip, false);
  const gated = [];
  for (const n of nums) {
    try {
      const pr = c.viewPr(n);
      if (landAction(pr, c.readLedger()) !== 'check') refuse(`#${n} is already merged`);
      c.trustGate(pr); c.draftGate(pr, n, '');
      const h = checkHead(pr, c.fetchRef(`pull/${n}/head`));
      if (!h.ok) refuse(h.reason);
      const d = await c.decide(pr, mainTip);
      if (!d.ok) refuse(`#${n}: ${d.reason}`);
      gated.push({ n, ok: true, pr, head: pr.headRefOid.toLowerCase(), via: d.via });
    } catch (e) { gated.push({ n, ok: false, reason: e instanceof Refusal ? e.message : why(e) }); }
  }
  const plan = planBatch(gated);
  for (const x of plan.dropped) say(`#${x.n} dropped from the batch: ${oneLine(x.reason)}`);
  if (plan.mode === 'none') refuse('no PR in the batch passed its gates; nothing merged');
  const chain = [];
  let lost = plan.dropped.length; // PRs of the request that will not land: the run exits non-zero when any
  let tip = mainTip;
  try {
    for (const g of gated.filter((x) => x.ok)) {
      let tree;
      try { tree = c.treeOf(tip, g.head); } catch (e) { say(`#${g.n} dropped from the batch: ${e.message}`); lost += 1; continue; }
      chain.push({ ...g, tree, parent: tip });
      tip = c.landCommit(chain.map((x) => x.n).join('-'), tree, tip, g.head);
    }
  } catch (e) { say(`batch: could not build the land commits (${why(e)})`); chain.length = 0; }
  let rest = plan.keep;
  if (chain.length >= 2) {
    rest = chain.map((x) => x.n);
    try {
      const steps = prefixes(chain); // every prefix on its own ref, waited on side by side: one CI cycle of wall clock
      const got = await Promise.allSettled(steps.map(async (s) => ({ id: s.id, ...(await c.ciLand(s.id, s.parent, s.head, s.tree)) })));
      const refused = got.find((g) => g.status === 'rejected');
      if (refused) throw refused.reason;
      const proof = batchProof(got.map((g) => g.value));
      if (c.dry) return say(`dry run: would merge ${rest.map((n) => `#${n}`).join(', ')} in that order once CI proves each prefix of land commit ${tip.slice(0, 8)} (land/${steps.at(-1).id}); merged nothing`);
      if (proof.ok) rest = await mergeBatch(c, chain, mainTip, got.map((g) => g.value.note));
      else { console.error(`land: no CI proof (${oneLine(proof.reason)})`); say('batch red; landing one at a time'); }
    } finally { c.ciClean(); }
  }
  lost += await singles(c, rest);
  if (lost) refuse(`${lost} of ${nums.length} PRs did not land`);
}
// Merges the chain's PRs in order, each as the single path does. Before each one: decided again, re-read, and main exactly where the last merge left it. GitHub's
// merge of PR k onto the merge commit of PR k-1 has the tree CI proved for L(k); the tree read back after each merge must equal it, or the batch stops and refuses.
// Returns the PRs not merged.
async function mergeBatch(c, chain, mainTip, notes) {
  const { say, refuse, Refusal, oneLine } = c;
  let expect = mainTip;
  for (const [i, p] of chain.entries()) {
    let stop = '';
    try {
      const again = await c.decide(p.pr, mainTip);
      if (!again.ok) stop = `#${p.n}: ${again.reason}`;
      else {
        const fresh = c.viewPr(p.n);
        c.trustGate(fresh); c.draftGate(fresh, p.n, '');
        const h = checkHead(fresh, p.head); // open, based on main, head still the tested one: the single path's check on the fresh read
        if (!h.ok) refuse(h.reason);
        const s = checkBatchStep({ expect, tip: c.fetchRef(c.MAIN_REF), head: p.head, headNow: fresh.headRefOid });
        if (!s.ok) stop = `#${p.n}: ${s.reason}`;
      }
    } catch (e) { if (!(e instanceof Refusal)) throw e; stop = e.message; }
    if (stop) { say(`batch: stopped before #${p.n} (${oneLine(stop)}); the rest land one at a time`); return chain.slice(i).map((x) => x.n); }
    const { sha, merged } = c.doMerge(p.n, p.head, p.tree, `${p.via}; batch ${notes[i]}`);
    if (merged !== p.tree) refuse(`#${p.n} merged as ${sha} but its tree is not the tested batch tree (${p.tree} tested, ${merged} merged); the LANDED row carries TREE-MISMATCH. Nothing else was merged; look at it now`);
    say(`#${p.n} landed as ${sha} (batch)`);
    expect = sha;
  }
  return [];
}
// The single path, PR by PR in order; returns how many were refused. This checkout moves to main's new tip between them (fast-forward only), as the operator would, since land demands a checkout at main's tip.
async function singles(c, list) {
  let bad = 0;
  for (const n of list) {
    try {
      const mainTip = c.fetchRef(c.MAIN_REF);
      if (c.out('git', ['rev-parse', 'HEAD']) !== mainTip) c.out('git', ['-c', 'core.hooksPath=/dev/null', 'merge', '--ff-only', mainTip]);
      await c.landOne(n, { bootstrap: false });
    } catch (e) { if (!(e instanceof c.Refusal)) throw e; console.error(`land: refused: ${e.message}`); bad += 1; }
  }
  return bad;
}
