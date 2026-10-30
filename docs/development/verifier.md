# Verifier protocol

A verifier is a fresh-context subagent. It never sees the builder's reasoning. It gets four things:
- the PR number and head SHA;
- the feature (from `feature_list.json`) or the unit's brief;
- the repo at that head;
- this file.

It runs on a different model from the builder.

## What it does, in order
1. **Reproduce.** From main's checkout run `make verify-preflight PR=<n>` (needs `JM_REPO`; `landing.md`): the same
   sandboxed merge-result preflight land runs. Read the PR's code freely, but never execute it outside that
   sandbox. Then the feature's `test` command, likewise only inside the sandbox: if it cannot run there, report
   `verifier-blocked` rather than run the PR's code outside it. A red result ends the review with `verifier-failed`.
2. **Acceptance.** Walk each step of the feature or brief against what the test actually asserts.
   A step that no assertion covers is a finding.
3. **Review.** Apply Pocock `code-review` to the diff against base `main`. Correctness comes first; then
   the contract (schema to generated code, server-stamped fields, token scope).
4. **Deslop.** Apply pstack `deslop` (`.agents/skills/deslop/SKILL.md`, base `main`). Look for dead code,
   needless abstraction, comments that restate code, and over-defensive guards.
5. **Subtract.** Answer two questions: could the diff be smaller, and does it delete anything it made
   obsolete?
6. **Entropy.** Report lines added and removed (`git diff --shortstat origin/main...HEAD`) and the LOC
   lines printed by `make entropy`.

## Verdict
The last lines of the report are always these, in this order:
```
LEDGER <pr> <head-sha> <unit-test-verified|live-ui-verified|type-check-only|verifier-blocked|verifier-failed>
FINDINGS <n blocking> <n non-blocking>
GATE: PASS | GATE: FAIL
```
- `GATE: PASS` requires zero blocking findings.
- The verifier never pushes, comments on GitHub or merges. The coordinator transcribes the report into the ledger
  row (`<pr> <head> PASS <model>-verifier <when> <note>`, `landing.md`): the `GATE:` line becomes the verdict, the
  verifier's model becomes `by`, and the `FINDINGS` line goes in `note`. `make land` reads the row. The coordinator
  writes no verdict the report does not carry.
- A protected-path PR still needs the reviewer's pod verdict at the same head
  (`landing.md`).
