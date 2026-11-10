# 0012 · Restart: the repo is the journal app, in Rust

Status: Accepted, by the owner's ruling, 2026-11-10: "i dont care about old code in jellymoon, we never run the app e2e
at all", and "feel free to do rewrtie the app if needed, remove inefficient stuff, improve rule/md/docs prompts, remove
uncessary part ... we want to write clean code as well". Asked about a server later, the owner wants room kept for one.

Until this note the repo held a different product: a shared space for people and their agents, with a Go daemon, two
servers, a web client and a schema generator. It was never run end to end by a person. [ADR 0011](0011-the-journal-app.md)
decided the journal app; this note makes the journal app the whole repo and says what was removed to get there.

## Decisions

1. **Rust only, one Cargo workspace.** No Go, Bun, Node or TypeScript. The shell is Rust already (ADR 0011, section 4),
   so a second language would mean two toolchains, two test runners and a wire format between them for one local app.
   For a user there is one process: the app links core (`crates/jellymoon-core`) as a library. The `jellymoon` binary
   is the same core behind commands that print JSON. All logic lives in core and is proved at the binary's commands;
   the shell only draws page data. That keeps every rule testable with no display, and keeps the shell replaceable.
2. **The contract is Rust types.** Page data and the reader's output are serde structs in core, with example JSON files
   that tests round-trip. A schema folder and a code generator earned their place when two languages shared the shapes;
   with one language the types are the single source and serde refuses what does not fit.
3. **The old product is removed, not archived in the tree.** A new contributor should see one small project. The old
   code stays reachable at the tag `harness-final` (commit `df20848`).
4. **The hosted instance is cut off from this repo before anything else changes.** Its deploy config, its workflow and
   the server code went in one commit, so no later push to `main` can start a deploy or a rollback. Removing them does
   not stop the running service; that is the owner's separate decision, and
   [the runbook](../runbooks/old-hosted-instance.md) keeps what is needed to do it.
5. **Landing is ordinary.** A pull request is squash-merged when CI is green on its head and a fresh-context review has
   posted `GATE: PASS` with its findings as a comment at that head. The review is a subagent that did not see the
   builder's reasoning, on a different model where possible. Blocking findings are fixed and reviewed again. Only the
   coordinator merges; workers never push or merge. The land tool, the verdict ledger and the verdict posted to another
   service are gone: they guarded a multi-tenant server, and cost more lines than the app they would now guard. One
   trust rule stays: code from an outside contributor's pull request is read, never run on a maintainer's machine,
   until a maintainer has reviewed it.
6. **The loop survives, slimmer.** One feature at a time from `feature_list.json`; a test first at the binary's
   boundary, and a unit test alone never marks a feature passing; only `passes` changes in the list from now on; lessons
   land in the same pull request as a test, lint or doc; handoff files in `progress/`. These are what kept long agent
   sessions honest, and none of them depended on the old product. `AGENTS.md` and
   [workflow.md](../development/workflow.md) hold them.
7. **The journal's privacy rules are repo rules.** Session logs are untrusted input and private. Fixtures are synthetic,
   never copied from a real log. The reader keeps only what a person typed. Credential-shaped text is replaced before it
   leaves the process. The reading call runs with every tool off. Text from a model or a log is drawn as text and never
   run. They come from ADR 0011 sections 1 to 3; stating them as rules lets a review fail a change on them.
8. **Windows from the first commit.** The app supports the Mac first and Windows after (ADR 0011, section 4). CI runs
   format, lints, the repo's own check and the tests on macOS, Linux and Windows, because a Windows break found late is
   a port and one found on the day it lands is a one-line fix.
9. **Room for a server.** Nothing server-side is built until a feature needs it. When one does (a friend's page is the
   likely first), it is a crate in the same workspace that reuses core's types, so the contract stays one set of types
   and no second repo or language appears.

## Tooling and size (restated from the removed ADR 0001)

- Rust is pinned in `rust-toolchain.toml`. `init.sh` installs it and a checksum-checked `gitleaks` into `.tools/`,
  inside the repo. Nothing is installed globally. `.tool-versions` is gone: it would be a second pin for one tool.
- A file is at most 400 lines and a function at most 60. Each crate has a LOC budget in `entropy.toml`.
  `cargo xtask check` enforces the file and crate numbers, the dependency direction (core, then the binary; the kit,
  then the app; core and the binary never depend on GPUI) and that no tracked file names a machine-local path; clippy
  enforces the function length.
- A budget grows only in the pull request of the feature that needs it, with the measured count and the reason in that
  pull request. Never for a refactor and never by deleting tests. The budgets exist because agent-written code grows
  without bound when nothing pushes back.
- A new dependency or a new crate gets a short ADR note: what it is for and what was rejected. Versions are exact in
  `Cargo.lock`.

## What was removed, and where it remains

All of it is at the tag `harness-final` and nowhere in the tree:

- the product: `daemon/` (Go), `server/`, `worker/`, `podcore/`, `web/`, `web-e2e/`, `hosted/`, `ee/`, and the
  black-box suite `e2e/`;
- the contract machinery: `schema/`, `gen/`, the generated protocol doc;
- the tooling: `scripts/` (the land tool, the generator, the private-names check, the size checks), `package.json` and
  its lockfile, `go.mod`, `entropy.json`, `.tool-versions`, the Go setup action and the deploy workflow;
- the records: ADRs 0001 to 0010, the design papers, the hosted runbook, 74 handoff files and the old `progress.md`,
  the standing orders in `.loop/`, and 23 features of the old product.

## What was kept

ADR 0011 and features F26 to F31, with F32 (themes) and F33 (install) added; the reader and reading-call docs; the
vendored stage skills in `.agents/skills/`; the sign-off rule and its CI job; the secret scan; the push fence for worker
worktrees; the licence. Three facts from ADRs 0005 and 0007 that F30 needs (the agent as an ACP child with an
allow-listed environment, cancel before any signal, permission refused by default) moved into ADR 0011, section 7.

## Consequences

- The behaviour of `jellymoon day` and `jellymoon page` is re-expressed in Rust from their docs and the old tests, not
  ported line by line. Until F26 and F27 pass again, nothing in the repo is proved.
- The private-names check that ran in the old preflight is gone with `scripts/`. `cargo xtask check` covers machine-local
  paths only; names and private identifiers are a review item.
- The required checks on `main` change to this repo's three CI jobs. An admin edits the ruleset when this lands.
