// Entropy gate (docs/development/workflow.md): size budgets, TODO ids, LOC budgets, Go import
// boundaries, and the exceptions ratchet. Budgets live in entropy.json.
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import ts from 'typescript';

const cfg = JSON.parse(readFileSync('entropy.json', 'utf8'));
const errors = [];
const isGenerated = (f) => cfg.generated.some((g) => f === g || f.startsWith(g));
const files = execFileSync('git', ['ls-files', '--cached', '--others', '--exclude-standard'], { encoding: 'utf8' })
  .split('\n').filter((f) => /\.(ts|tsx|mjs|js|go)$/.test(f) && !isGenerated(f));

function tsFunctions(file, text) {
  const src = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true);
  const out = [];
  const visit = (node) => {
    if (ts.isFunctionLike(node) && 'body' in node && node.body) {
      const name = node.name?.getText(src)
        ?? (ts.isVariableDeclaration(node.parent) ? node.parent.name.getText(src) : '<anonymous>');
      const start = src.getLineAndCharacterOfPosition(node.getStart(src)).line;
      const end = src.getLineAndCharacterOfPosition(node.getEnd()).line;
      out.push({ name, lines: end - start + 1 });
    }
    ts.forEachChild(node, visit);
  };
  visit(src);
  return out;
}

// gofmt puts every top-level func's opening line and closing brace at column 0.
function goFunctions(text) {
  const out = [];
  let open = null;
  text.split('\n').forEach((line, i) => {
    const m = /^func (?:\([^)]*\) )?([A-Za-z0-9_]+)/.exec(line);
    if (m && !line.endsWith('}')) open = { name: m[1], start: i };
    else if (open && line === '}') { out.push({ name: open.name, lines: i - open.start + 1 }); open = null; }
  });
  return out;
}

const loc = Object.fromEntries(Object.keys(cfg.locBudget).map((k) => [k, 0]));
for (const file of files) {
  const text = readFileSync(file, 'utf8');
  const lines = text.split('\n').length;
  if (lines > cfg.maxFileLines && !cfg.exceptions.files[file]) errors.push(`${file}: ${lines} lines > ${cfg.maxFileLines}`);
  const fns = file.endsWith('.go') ? goFunctions(text) : tsFunctions(file, text);
  for (const fn of fns) {
    if (fn.lines > cfg.maxFunctionLines && !cfg.exceptions.functions[`${file}:${fn.name}`]) {
      errors.push(`${file}:${fn.name}: ${fn.lines} lines > ${cfg.maxFunctionLines}`);
    }
  }
  if (!cfg.markerScanSkip.includes(file)) text.split("\n").forEach((line, i) => {
    if (/\b(TODO|FIXME|XXX)\b/.test(line) && !/\b(TODO|FIXME|XXX)\((F\d+|TASK-\d+)\)/.test(line)) {
      errors.push(`${file}:${i + 1}: TODO needs a row id, e.g. TODO(F07)`);
    }
  });
  for (const [area, b] of Object.entries(cfg.locBudget)) if (b.paths.some((p) => file.startsWith(p))) loc[area] += lines;
}
for (const [area, b] of Object.entries(cfg.locBudget)) {
  console.log(`LOC ${area.padEnd(6)} ${String(loc[area]).padStart(5)} / ${b.max}`);
  if (loc[area] > b.max) errors.push(`LOC budget: ${area} ${loc[area]} > ${b.max}`);
}

// entropy.json is the only place the caps are written; print their total here so nobody restates it by hand.
console.log(`caps total: ${Object.values(cfg.locBudget).reduce((sum, b) => sum + b.max, 0)}`);

if (files.some((f) => f.endsWith('.go'))) {
  const mod = execFileSync('go', ['list', '-m'], { encoding: 'utf8' }).trim();
  const pkgs = execFileSync('go', ['list', '-f', '{{.ImportPath}} {{join .Imports " "}}', './...'], { encoding: 'utf8' });
  for (const row of pkgs.trim().split('\n')) {
    const [pkg, ...imports] = row.split(' ').filter(Boolean);
    const rule = cfg.goImports[pkg.replace(`${mod}/`, '')];
    if (!rule) continue;
    for (const imp of imports) {
      const std = !imp.split('/')[0].includes('.');
      const ok = (std && rule.allow.includes('std')) || rule.allow.some((a) => imp === `${mod}/${a}`);
      if (!ok) errors.push(`import boundary: ${pkg} may not import ${imp}`);
    }
  }
}

// Ratchet: exceptions only shrink, unless the new one cites an ADR note. `make land` names the base (JM_ENTROPY_BASE, main's tip as
// fetched); a named base that cannot be read fails loudly (`none`: the bootstrap land, main has no entropy.json yet). Unnamed, origin/main is used, and a missing one means no base yet.
const named = process.env.JM_ENTROPY_BASE;
if (named !== 'none') try {
  const base = JSON.parse(execFileSync('git', ['show', `${named ?? 'origin/main'}:entropy.json`], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }));
  for (const kind of ['files', 'functions']) {
    for (const [key, reason] of Object.entries(cfg.exceptions[kind])) {
      if (!(key in base.exceptions[kind]) && !reason.includes('docs/adr/')) errors.push(`new ${kind} exception ${key} must cite a docs/adr/ note`);
    }
  }
} catch { if (named) errors.push(`ratchet: cannot read entropy.json at JM_ENTROPY_BASE=${named}; the ratchet would switch off`); }

// A cap changes only with a new note in docs/entropy/caps/ in the same diff (ADR 0001). The base is the merge-base of HEAD and
// the named base (JM_ENTROPY_BASE, else origin/main). A shallow clone, or a base that cannot be read, skips only this check
// and says so; the ratchet above already fails loudly for a named base it cannot read. CI checks out with fetch-depth 0.
function git(...args) { return execFileSync('git', args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }); }
if (named !== 'none') try {
  if (git('rev-parse', '--is-shallow-repository').trim() === 'true') throw new Error('shallow clone');
  const mergeBase = git('merge-base', 'HEAD', named ?? 'origin/main').trim();
  const baseCaps = JSON.parse(git('show', `${mergeBase}:entropy.json`)).locBudget;
  const changed = [...new Set([...Object.keys(baseCaps), ...Object.keys(cfg.locBudget)])]
    .filter((area) => baseCaps[area]?.max !== cfg.locBudget[area]?.max);
  const noteDir = 'docs/entropy/caps/';
  const added = [
    ...git('diff', '--name-status', '--diff-filter=A', mergeBase).split('\n').map((l) => l.split('\t')[1]),
    ...git('ls-files', '--others', '--exclude-standard', noteDir).split('\n'),
  ].filter((f) => f?.startsWith(noteDir));
  if (changed.length && !added.length) {
    errors.push(`cap changed for ${changed.join(', ')} without a new note: add a file under ${noteDir} in this diff (ADR 0001)`);
  }
} catch (e) { console.log(`cap-note check skipped: cannot read the base of this diff (${e.message.split('\n')[0]})`); }

if (errors.length) { console.error(errors.join('\n')); process.exit(1); }
console.log(`entropy ok: ${files.length} files`);
