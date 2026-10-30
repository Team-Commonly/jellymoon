# Jellymoon progress

Newest entry first. Each entry has four parts: what changed, the evidence (a test command and its
result, or the PR), what's next, and anything surprising. Keep each entry under 15 lines; the detail
goes in the PR.

## 2026-10-30 · P08 one reading of the scan, recipe pinned (unit P08, round 19)
- **Changed:** `scanResult` in land-decide.mjs is the one reading of the range scan (clean only on exit 0; hit only on a nonzero exit with `exit status 3`; anything else, a zero exit that printed the hit line included, is a failure), used by `rangeScan` and by every fixture. The `secrets-range` recipe and the go/gitleaks definitions are text-pinned. Every range shape runs with no ref at the head and with refs (a branch and a `refs/remotes` ref); the evil-merge block is land-shaped. The scan runs under an allowlist env (`scanEnv`): no `MAKEFLAGS`, no operator gitleaks or git config. Docs: bootstrap checkout must be the PR head, step order, step 6.
- **Evidence:** check-land 163, check-land-e2e 98, check-review-gate 10 verdict + 4 symlink + 3 history + 2 portable + 3 evil-merge + 2 recipe pin + 15 range.
- **Next:** land #10 with `--bootstrap`.
- **Surprising:** the two fixture readers and land each had their own reading of the same result, so a recipe that exited 0 on a hit passed every gate.

## 2026-10-30 · P08 range control, land-shaped (unit P08, round 18)
- **Changed:** the range cases now model a normal land (checkout on main, PR head reachable by SHA only) in three shapes: linear, a merged side branch, main moved after branching. `--bootstrap` requires HEAD to be the PR head. A pod page holding a non-object refuses in words. Doc fixes: built-in allowlist cited by reference, stale wording in GLOSSARY, INDEX, ADR and the design doc.
- **Evidence:** check-land 155, check-land-e2e 93, check-review-gate 10 verdict + 4 symlink + 3 history + 2 portable + 3 evil-merge + 7 range.
- **Next:** land #10 with `--bootstrap`.
- **Surprising:** a fixture left on the PR branch passed four recipe mutants (`HEAD`, `--all`, `--first-parent`, `--ancestry-path`) that a main checkout exposes.

## 2026-10-30 · P08 range control and pod paging (unit P08, round 17)
- **Changed:** restored the real-recipe range cases (add then delete a token; marker ignored; clean range), e2e cases for every pod paging guard, an exact PR url check, a non-list pod reply refusing in words, and land-decide reached by a non-literal path so knip sees its exports.
- **Evidence:** check-land 153, check-land-e2e 87, check-review-gate 10 + 4 + 3 + 2 + 3 + 3; core LOC 1833.
- **Next:** as round 18.

## 2026-10-30 · P08 one review per head, scans only refuse (unit P08, round 16)
- **Changed:** every land needs the ledger PASS and the pod verdict; no path rule or scan result waives either. Deleted the inert rule, `needsVerdict`, the neutral-name re-scan, the range-blob enumeration, `touchesProtected` and the path cases. `make secrets-range` now exits 3 on a finding; a hit or a failed scan refuses. Reviewers env is checked before the token read ("malformed" if no valid entry); `msgUser` takes only a string `_id`.
- **Evidence:** check-land 153, check-land-e2e 73, check-review-gate 10 verdict + 4 symlink + 3 history + 2 portable + 3 evil-merge; core LOC 2103 to 1787.
- **Next:** land #10 with `--bootstrap`.
- **Surprising:** `go run` reports a child's exit 3 as "exit status 3" and exits 1 itself, so land reads stderr to tell a finding from a failed scan.

## 2026-10-29 · P08 trust gate, not sandbox (unit P08, round 5)
- **Changed:** deleted the macOS sandbox, its proof, the platform check, `--reviewer-gate-unavailable`, `--owed` and `REGATE-OWED`; land and verify-preflight run PR code only for a same-repo PR whose author and every push-capable account are in `JM_TRUSTED_LOGINS`; the checkout must be at main and clean with no credentials in its git config; any FAIL at a head blocks it; failed check runs veto; an added `gitleaks:allow` line makes a PR protected; preflight output is quoted; check-gen is proved to bite. **Evidence:** check-land 88 cases, core LOC 1455 to 1414 (max 1454). **Next:** land #10 with `--bootstrap`; `ci.yml` stays off. Core LOC max raised from 1454 to 1500 (ADR 0001's cap) so security tests are not deleted to fit.

## 2026-10-29 · P08 sandbox and inert docs (unit P08, round 4)
- **Changed:** protection is now an allow-list of inert paths (`progress.md`, doc extensions under `docs/`; `docs/**/*.go` is protected); the preflight sandbox denies the real home, unix sockets and the keychain service, writes only to its throwaway dir, and is proved by a four-probe check before every land and in `make test`; `make verify-preflight PR=<n>` gives verifiers the same sandbox; deleted the GitHub review-gate code (`review-gate.mjs`, `review-gate.yml`); merge without repo hooks, scripts/ must equal main's, one PR number, real worktree-add reason, atomic stale-lock takeover, `MERGED-UNVERIFIED` for out-of-band merges.
- **Evidence:** check-land 107 cases (sandbox proof included), check-review-gate 11 + 33; core LOC budget 1500 to 1455.
- **Next:** land #10 with `--bootstrap`; the PR template still lists the old protected paths and `ci.yml` still names the deleted workflow (neither in this unit's scope).

## 2026-10-29 · P08 final fixes (unit P08, round 3)
- **Changed:** every path but `docs/` and `progress.md` is now protected (renames count both sides); the preflight runs in a macOS `sandbox-exec`; pod paging dedupes by id and fails closed at a boundary; a bad token file is never echoed; `--dry-run` writes no ledger row; both workflows are `workflow_dispatch` only; CODEOWNERS deleted; one wording for who writes verdict rows; internal labels scrubbed.
- **Evidence:** `make preflight` green in the stripped shell; check-land 89 cases (sandbox proof included), check-review-gate 44 + 23.
- **Next:** land #10 with `--bootstrap`; the PR template still lists the old protected paths (not in this unit's scope).

## 2026-10-29 · P08 review fixes (unit P08, round 2)
- **Changed:** land simplified to one PR based on `main` (no `--stack`, chains or FIXED-FWD). It now tests main plus
  the head in a throwaway worktree, takes changed files from the local `git diff --name-status -M`, re-reads the
  ledger and pod right before the merge, refuses if main moved, and takes a lock. Pod verdicts count only with a
  first line `jellymoon#<pr> <head>`. Ledger fails closed (unknown verdicts, any FAIL, verifier labels).
  Preflight cleanup can no longer turn a pass into a refusal. `land.mjs`, `Makefile`, `init.sh` and tools.sh are protected.
- **Evidence:** `make preflight` green in the stripped shell; the three surviving mutations are now killed.
- **Next:** the first land of the gate uses `--bootstrap` and is judged by people; verify the reviewer usernames.

## 2026-10-29 · local landing (unit P08)
- **Changed:** `make land PR=<n>` (scripts/land.mjs, pure decisions in land-decide.mjs): clean-shell preflight at
  the exact head, verifier ledger line, and for protected paths the review's pod verdict matched by
  the author's immutable user id. Both workflows disabled (headers only). Temporary `--reviewer-gate-unavailable` second
  verifier plus `--owed`. P07 round-2 carry-overs fixed here: verdict is the last non-empty line (N1), credit
  lines normalised (N2), 14 new gate cases (N3), check-portable case-insensitive again (N4), accepted reviewers in operator env only (N5); B1 is moot with CI off and is stated in workflow.md, AGENTS.md and the ymls.
- **Evidence:** `make test` runs 44 gate cases and 70 land cases (see the round-2 entry below); the accept-any-username mutation turns two red.
- **Next:** the coordinator's live dry-run; set JM_VERDICT_REVIEWERS to the operator's pod user id.
- **Surprising:** the clean shell has no `gh` or `node`, so land runs under bun and the dry-run needs a normal PATH.

## 2026-10-29 · loop fixes from the PR #1 and #2 verifiers (unit P07)
- **Changed:** `make` finds `.tools/bin` by path (GNU Make 3.81 ignores the exported PATH for command lookup).
  Review gate decision moved to `scripts/review-gate.mjs`, run from the base branch, fail closed (multi-credit
  blocks, fences and blockquotes ignored, CHANGES_REQUESTED blocks, renames checked), reviewers in
  the operator reviewer env (JM_VERDICT_REVIEWERS). `whoami` agent/runId optional; verificationUri, Invite.url, mcpUrl https only
  (mcpUrl: loopback http with optional port). Fixtures: PostInput invalids assert their rule via `expect.json`.
  Docs: worktree init, fence, gate limits, glossary (gate advisory), worker/ and e2e/ marked not yet created.
- **Evidence:** PATH-stripped `make preflight` green; `make test` runs the 31 gate cases and 7 path cases.
- **Next:** F02. A server-stamped verdict check replaces the credit-line pointer. The first merge of the gate
  into `main` fails closed and is pressed by hand with the Opus `GATE: PASS` checked at the head.
- **Surprising:** `SHELL := /bin/bash` does not help Make 3.81; only calling tools by path does. A gate that
  runs the PR's own judge can be made green by the PR.

## 2026-10-29 · claim.released and cleanup (unit P05)
- **Changed:** s2c event `claim.released` (`ClaimReleased`, `ClaimReleaseCause`: expired, released, run_ended,
  stopped) with fixtures; ADR 0002 names the cause each path emits. Closed 12 verifier findings: glossary and
  three `ux-concepts.html` lease lines, gen formats Go in memory (a parse failure writes nothing, `gofmt -w`
  step gone, `write` is now `queueOutput`), TaskKind and lease-rule text restored, `run.stop` re-send, cancelled
  is final after `run/end`, task_release auth `run|user`.
- **Feature text:** F10 gained one step (claim.released per way a claim ends). No other feature changed.
- **Evidence:** `make gen` twice is clean; `make preflight` green. Not enforced by schema: `by` required for
  released/stopped (no oneOf); the Worker tests it.
- **Next:** F02.

## 2026-10-29 · human release and stop (unit P04)
- **Changed:** operations `task_release` and `run_stop`, s2c event `run.stop`, their input/output defs and
  fixtures; `x-rules` in `operations.json`, `events.json` and `task.json` say who may call each and that a
  person-released claim is not restored. ADR 0002 gains "Recourse". Nothing deleted.
- **Feature text:** appended one step to F10 (owner releases the claim; the run gets forbidden) and one to
  F13, the feature that tests `POST /v1/runs/:run/end` (pod owner stops a live run). No other text changed.
- **Evidence:** `make gen` twice is clean; `make preflight` green. The fixture harness checks data, not
  direction, so there is no fixture for a c2s `run.stop`; direction is only in `x-variants` and an x-rule.
- **Next:** F02. **Open:** the hang is still undetectable by the pod; a person must notice it.

## 2026-10-29 · gen is all-or-nothing, claims and leases (unit P03)
- **Changed:** `make gen` renders and checks everything, then writes; a failing run leaves the tree as it
  was. It also fails on an auth value outside `none|user|computer|run` (joined with `|`). The nine P02
  nits are closed (error-body field, `Envelope.data`, `ts`/`online`, "the poster", TaskKind text, GitHub
  slugs). New `docs/adr/0002-claims-and-leases.md`; `task.json` gets `x-rules` for it.
- **Feature text:** F10 only, as the brief sanctioned: description and steps 4-5 now say the lease lives as
  long as its run (heartbeat renews it). No other feature changed; `passes` is untouched.
- **Evidence:** mutations (missing description; duplicate anchor with a valid schema change; `runn` and
  `computr|user` auth) fail with the tree byte-identical. Shapes match `p02-schema-text` once `description`,
  `x-errors` and `task.json` `x-rules` are stripped. `make gen` twice is clean; `make preflight` green.
- **Next:** F02. **Surprising:** the 60 s lease and the 2 h run cap are still guesses (ADR 0002 lists them).

## 2026-10-29 · schema text (unit P02)
- **Changed:** every `$def` and every property that is not a bare `$ref` has a plain description; each
  `ErrorCode` carries `x-errors` (HTTP status and meaning). `make gen` fails and names each gap, a
  description over 200 characters, or an `x-errors` key set that differs from the enum. Gen emits
  `ERROR_STATUS` (TS) and `ErrorStatus` (Go). Also: the anchor check compares sets of names (one def
  dropped and another doubled now fails), the "only the fields listed" sentence in `docs/protocol.md`
  names its exception (`Envelope.data`), each Rules heading is unique, auth labels show the raw value and
  its token, the errors table links to the doc's own headings, and the contents line comes from the one
  title list. Nothing deleted beyond reworded lines.
- **Evidence:** schema with every `description` and `x-errors` key removed is identical before and after;
  `make gen` twice leaves the tree clean; mutation runs fail as intended; `make preflight` green.
- **Open:** statuses other than `gone` (410) and the meanings of `slow_down`, `not_found` and
  `invalid_input` are not decided by `docs/design/protocol.md`; the review should confirm them.

## 2026-10-29 · protocol reference doc (unit P01, rebased)
- **Changed:** `make gen` also writes `docs/protocol.md`, a readable reference of the whole protocol
  (every definition, every event, every operation with method, path, auth and MCP name, error codes, rules as
  sentences). `scripts/protocol-doc.mjs` renders it from the data `scripts/gen.mjs` already loads;
  `make check-gen` diffs the doc too. Rebased onto operations.json: the doc lists operations, not tools.
  No new dependency; nothing deleted.
- **Evidence:** `make gen` twice leaves the tree unchanged; a schema description edit makes
  `make check-gen` fail until regenerated; `make preflight` green.
- **Open:** a definition without a schema description shows an empty Notes cell; fixing that is a schema edit.

## 2026-10-29 · Jellymoon names applied (build session)
- **Changed:** decided at the protocol review: MCP adapter prefix `jm_`, tokens `jmu_`/`jmc_`/`jmr_`,
  schema `$id` on `jellymoon.dev/schema/v1`, CLI binary `jellymoon` (alias `jm`), package `jellymoon`.
  Feature step text changed only for these names (F03, F08, F09, F10, F11, F12, F13, F15); no step's
  meaning changed. This resolves the naming hold noted below.
- **Evidence:** `make preflight` green.

## 2026-10-29 · two founder directions applied to the schema (build session)
- **Changed:** asks carry a required grounded `context` (happened, checked, stakes, recommendation) and
  each option a `consequence`; every claim cites `tc_` tool-call or message ids, and a post whose
  numbers aren't in its cited evidence is refused with `ungrounded`. The pod stamps tool calls itself
  (`run.toolcall` from the daemon, MCP calls it sees); agents read them via the `evidence` operation.
  `schema/tools.json` became `schema/operations.json`: plain HTTP operations are the contract, and
  MCP (`x-mcp-prefix`) and the CLI are generated adapters.
- **Evidence:** `make preflight` green; 76 fixtures, 3 new negatives.
- **Next:** the grounding x-rules need Worker tests when F-posting lands; features F08/F15 still quote
  MCP-first wording, changed in the naming PR after the design review.

## 2026-10-29 · portability fixes from loop review (build session)
- **Changed:** the repo now works from a fresh clone anywhere. Stage skills are vendored in
  `.agents/skills/` (pins in `docs/skills/SOURCES.md`); Go and Bun are pinned in `.tool-versions` and
  `init.sh` installs them into `.tools/`; `make portable` (preflight, pre-commit, CI) fails on a symlink
  out of the repo or a machine-local path. The merge rule says what actually runs: coordinator merges on
  green CI plus a ledger verdict, since GitHub Free gives a private repo no merge queue.
- **Evidence:** `./init.sh` green with `HOME` set to an empty dir and only system dirs on `PATH` (35 s
  cold, tools downloaded and checksum-verified). Mutation checks: an outside symlink and a `~/` path
  each fail `make portable`.
- **Surprising:** the first pilot passed its own setup on this machine while depending on a committed
  `node_modules` symlink and skill paths under the operator's home. A pilot proves portability only if
  it runs without the operator's environment, so pilots now run with a scratch `HOME`.
- **Open:** the MCP tool prefix and CLI binary name are an open naming question for the protocol
  review. The prefix is one constant (`x-tool-prefix` in `schema/tools.json`); feature text still
  quotes today's names.

## 2026-10-29 · initializer (build session)
- **Changed:** set up the long-running harness. `feature_list.json` holds 20 slice features as e2e
  tests (F01 passes). Also added `init.sh`, this file, the `AGENTS.md` loop, the PR template and
  `docs/development/workflow.md`.
- **Plan:** one builder session works top-down through the list. Two design docs run alongside as
  subagents (connectors, OSS-first; client options). F18 and F19 are blocked until the maintainers review those
  docs.
- **Evidence:** F01 is `make check-gen test`, green on #2977 (L0).
- **Next:** F02, the `make e2e` harness (wrangler dev, /healthz, E2E_MS).
- **Surprising:** the fleet's husky pre-commit hook doesn't apply here; `make hooks` installs
  this repo's own pre-commit. F17 needs a Cloudflare API token
  secret, an owner action.

## 2026-10-29 · repo move (build session)
- **Changed:** the build moved from the `v4` branch of the Commonly repo to its own private repo,
  `Team-Commonly/jellymoon` (the company stays Commonly). History was rebuilt clean:
  a fixture token that tripped gitleaks became an obviously fake one, and the Go module is
  `github.com/Team-Commonly/jellymoon`.
- **Held:** the CLI binary name (`commonly` in F08/F15) and the npm scope wait for the trademark check.
- **Surprising:** the org is on GitHub Free, so a private repo gets no rulesets or merge queue.
  Until that is decided, the coordinator merges only on green CI plus a ledger verdict.
