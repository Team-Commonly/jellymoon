// Decides whether the protected-path review gate is satisfied (called by .github/workflows/review-gate.yml,
// which runs the BASE branch's copy of this file, never the PR's). Fails closed. At the exact PR head:
//  - DISMISSED reviews are ignored; code fences and blockquotes are ignored when finding credit and GATE lines;
//  - a review is credited to N if a credit line "Written by <N>, a Commonly agent" names a listed reviewer N;
//  - a review credited to more than one listed name BLOCKS (a FAIL that quotes another credit line is not swallowed);
//  - for each listed reviewer only their latest credited review counts; its verdict is the review's LAST
//    non-empty line (see verdictOf). It passes only if that line is exactly GATE: PASS and the review state
//    is not CHANGES_REQUESTED;
//  - the gate passes iff at least one reviewer's latest review passes and none blocks.
// DISABLED with its workflow: scripts/land.mjs reuses touchesProtected, verdictOf and the reviewer
// list; there the server-stamped pod username decides, not a credit line.
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';

const CREDIT = /written by ([^,\n]+), a commonly agent/gi;

// The one protected-path rule, used by land and by the (disabled) workflow. Everything the preflight or the
// merge runs comes from the PR tree, so EVERY changed path is protected except documentation: paths under
// docs/ and the file progress.md. A rename counts both sides: docs/x -> scripts/x is protected.
const UNPROTECTED = /^(docs\/|progress\.md$)/;

// files: the PR's changed files as { filename, previous_filename? }.
export function touchesProtected(files) {
  return files.some((f) => [f.filename, f.previous_filename].some((n) => n && !UNPROTECTED.test(n)));
}

// One reviewer per line: `Name username` (the username is the pod's server-stamped one, used by
// scripts/land.mjs). A line with a single token is a name with no username.
export function readReviewerEntries(text) {
  return text.split('\n').map((l) => l.replace(/#.*/, '').trim()).filter(Boolean).map((l) => {
    const parts = l.split(/\s+/);
    return parts.length > 1 ? { name: parts.slice(0, -1).join(' '), username: parts.at(-1) } : { name: l, username: null };
  });
}

export function readReviewers(text) {
  return readReviewerEntries(text).map((e) => e.name);
}

// Credit-line text (GitHub path only): the body without fenced code and blockquote lines. A fence closes
// only on the marker that opened it.
function visibleText(body) {
  let fence = null;
  return body.split('\n').filter((line) => {
    const m = /^\s*(```|~~~)/.exec(line);
    if (m && !fence) { fence = m[1]; return false; }
    if (m && m[1] === fence) { fence = null; return false; }
    return !fence && !/^\s*>/.test(line);
  }).join('\n');
}

// Credit lines compared after dropping emphasis and links, with NBSP and fullwidth comma as plain ones.
const tidy = (t) => t.replace(/\[([^\]]*)\]\([^)]*\)/g, '$1').replace(/[*_`]/g, '').replace(/[\u00a0\u3000]/g, ' ')
  .replace(/[，､、]/g, ',').replace(/[ \t]+/g, ' ');

// The verdict is the LAST non-empty line, with HTML comments removed and emphasis and list or heading
// markers stripped. Only exactly `GATE: PASS` (case-insensitive) passes. Returns 'PASS', 'BLOCK', or null
// (nothing to judge). Fences play no part; an indented line is code and blocks.
export function verdictOf(text) {
  const last = text.replace(/<!--[\s\S]*?-->/g, '').split('\n').filter((l) => l.trim()).pop();
  if (last === undefined) return null;
  if (/^(\t| {4})/.test(last)) return 'BLOCK';
  const line = last.replace(/[*_`~]/g, '').trim().replace(/^(#{1,6}|[-+]|\d+\.)\s+/, '');
  return /^GATE: PASS$/i.test(line) ? 'PASS' : 'BLOCK';
}

export function decide(reviews, { head, names }) {
  const lower = names.map((n) => n.toLowerCase());
  const latest = new Map();
  const blockers = [];
  const at = (r) => Date.parse(r.submitted_at || 0) || 0;
  reviews.forEach((r, i) => {
    if (r.commit_id !== head || r.state === 'DISMISSED') return;
    const text = visibleText(r.body || '');
    const named = [...tidy(text).matchAll(CREDIT)].map((m) => m[1].trim().toLowerCase());
    const credited = [...new Set(named.filter((n) => lower.includes(n)).map((n) => names[lower.indexOf(n)]))];
    if (credited.length > 1) { blockers.push(`a review credited to ${credited.join(' and ')}`); return; }
    if (credited.length === 0) {
      if (/written\s*by/i.test(tidy(r.body || '')) && verdictOf(r.body || '') !== 'PASS') blockers.push('a non-PASS review with an unrecognised credit line');
      return;
    }
    const prev = latest.get(credited[0]);
    if (!prev || at(r) > at(prev.r) || (at(r) === at(prev.r) && i > prev.i)) latest.set(credited[0], { r, i });
  });
  const passing = [];
  for (const [name, { r }] of latest) {
    const verdict = verdictOf(r.body || '');
    if (verdict === 'PASS' && r.state !== 'CHANGES_REQUESTED') passing.push(name);
    else blockers.push(`${name}'s latest review (${r.state === 'CHANGES_REQUESTED' ? 'CHANGES_REQUESTED' : 'last line is not exactly GATE: PASS'})`);
  }
  if (blockers.length) return { pass: false, reason: `blocked by ${blockers.join('; ')}` };
  if (!passing.length) return { pass: false, reason: `no reviewer's latest review at ${head} is GATE: PASS` };
  return { pass: true, reason: `Opus gate found at head (${passing.join(', ')})` };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [reviewsFile, reviewersFile, filesFile] = process.argv.slice(2);
  if (!touchesProtected(JSON.parse(fs.readFileSync(filesFile, 'utf8')).flat())) {
    console.log('No protocol or security path touched; no review needed.');
    process.exit(0);
  }
  const names = readReviewers(fs.readFileSync(reviewersFile, 'utf8'));
  const reviews = JSON.parse(fs.readFileSync(reviewsFile, 'utf8')).flat();
  const { pass, reason } = decide(reviews, { head: process.env.HEAD, names });
  console.log(pass ? reason : `::error::protocol or security path touched; ${reason}; accepted reviewers: ${names.join(', ')}`);
  process.exit(pass ? 0 : 1);
}
