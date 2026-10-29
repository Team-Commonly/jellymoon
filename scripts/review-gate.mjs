// Decides whether the protected-path review gate is satisfied (called by .github/workflows/review-gate.yml,
// which runs the BASE branch's copy of this file, never the PR's). Fails closed. At the exact PR head:
//  - DISMISSED reviews are ignored; code fences and blockquotes are ignored when finding credit and GATE lines;
//  - a review is credited to N if a credit line "Written by <N>, a Commonly agent" names a listed reviewer N;
//  - a review credited to more than one listed name BLOCKS (a FAIL that quotes another credit line is not swallowed);
//  - for each listed reviewer only their latest credited review counts; its verdict is the last GATE line
//    (case-insensitive, markdown emphasis stripped). It passes only if that line is exactly GATE: PASS and the
//    review state is not CHANGES_REQUESTED. No GATE line, any other word, or CHANGES_REQUESTED blocks;
//  - the gate passes iff at least one reviewer's latest review passes and none blocks.
// KNOWN LIMIT: the credit line is a pointer, not an identity (see the workflow header).
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';

const CREDIT = /Written by ([^,\n]+), a Commonly agent/g;

const PROTECTED = /^(schema\/|worker\/src\/auth\/|worker\/src\/mcp\/|daemon\/internal\/(acp|run)\/|worker\/public\/install\.sh$|\.github\/workflows\/|\.github\/opus-reviewers\.txt$|scripts\/review-gate\.mjs$)/;

// files: the PR's changed files as GitHub lists them. A rename OUT of a protected path lists only the new
// name in `filename`, so previous_filename counts too.
export function touchesProtected(files) {
  return files.some((f) => [f.filename, f.previous_filename].some((n) => n && PROTECTED.test(n)));
}

export function readReviewers(text) {
  return text.split('\n').map((l) => l.replace(/#.*/, '').trim()).filter(Boolean);
}

// The body without fenced code and blockquote lines.
function visibleText(body) {
  let fenced = false;
  return body.split('\n').filter((line) => {
    if (/^\s*(```|~~~)/.test(line)) { fenced = !fenced; return false; }
    return !fenced && !/^\s*>/.test(line);
  }).join('\n');
}

function verdictOf(text) {
  const line = text.split('\n').map((l) => l.replace(/[*_`]/g, '').trim()).filter((l) => /^gate:/i.test(l)).pop();
  return line ? line.slice(5).trim().replace(/\s+/g, ' ').toUpperCase() : null;
}

export function decide(reviews, { head, names }) {
  const latest = new Map();
  const blockers = [];
  const at = (r) => Date.parse(r.submitted_at || 0) || 0;
  reviews.forEach((r, i) => {
    if (r.commit_id !== head || r.state === 'DISMISSED') return;
    const text = visibleText(r.body || '');
    const credited = [...new Set([...text.matchAll(CREDIT)].map((m) => m[1].trim()).filter((n) => names.includes(n)))];
    if (credited.length > 1) { blockers.push(`a review credited to ${credited.join(' and ')}`); return; }
    if (credited.length === 0) return;
    const prev = latest.get(credited[0]);
    if (!prev || at(r) > at(prev.r) || (at(r) === at(prev.r) && i > prev.i)) latest.set(credited[0], { r, i, text });
  });
  const passing = [];
  for (const [name, { r, text }] of latest) {
    const verdict = verdictOf(text);
    if (verdict === 'PASS' && r.state !== 'CHANGES_REQUESTED') passing.push(name);
    else blockers.push(`${name}'s latest review (${r.state === 'CHANGES_REQUESTED' ? 'CHANGES_REQUESTED' : verdict ? `GATE: ${verdict}` : 'no GATE line'})`);
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
