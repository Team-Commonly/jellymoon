# Standing gates on briefs

## B-ADR: a brief must not override an Accepted ADR
A brief once overrode an Accepted ADR twice without anyone noticing. The second time was "a refused command never
renews", against ADR 0002's "renews on any operation". It broke ADR 0003's restart guarantee, and a verifier caught it
only in the code.
- Every core or adapter brief carries a `GOVERNING ADRs` line. It quotes, verbatim, each Accepted ADR clause the
  brief relies on or touches, with file:line.
- The coordinator greps the Accepted ADRs for each subject a "Decided" rule touches, before the brief goes out.
- Every verifier brief points at the unit brief. The verifier checks each "Decided" rule against the Accepted ADRs,
  quoted or not, and FAILS the unit if a rule contradicts one. "The brief said so" is not a defence.
- An unratified ADR loses to an Accepted one. If a brief needs to change an Accepted ADR, that is a ruling: a new or
  amended ADR is accepted first, and then the brief cites it.

## Brief checklist (coordinator, before any brief or ruling goes out)
- Grep the Accepted ADRs for every subject a Decided rule touches (B-ADR). A cap number comes from ADR 0001's table, and
  raising one takes a dated note in 0001 itself, not in the unit's own ADR.
- A pure derivation (a wake id as seed + k) is checked against every minter that feeds it: monotonic ULIDs made
  seed + 1 the next command's seed.
- Every early return in podcore is checked against EVERY outcome rule in ADR 0002, not one clause: renewal,
  `unauthorized` for an ended run, `forbidden` for a non-member. One round skipped renewal; the next fixed it
  and still answered not_found to an ended run and a stranger. The tests for an ordering rule send malformed and
  prototype-key input too, because a well-formed-only test passes with the order broken.
- Never trade tests for a LOC cap: restore coverage first, then move the cap within the ADR. (Earned: a round cut 27
  check-land cases and 4 security mutants survived.)
- A ruling that adds state to the core (a messageId set) gets a bound and an old-saved-state check before it ships.
- A test of wiring must run the wiring. Source greps (`src.includes('trustGate(')`) pass with the result ignored: one round
  had 163 cases and 23 of 75 land.mjs mutants survived. Ask for an injectable I/O seam and end-to-end cases from the start.
- A scanner's blind spot is a class, not a case. Fixing "images" left NUL-text and the scanner's own path allowlist open;
  scan what you allow under names you choose.
- A subagent never runs `rm -rf` or `chmod -R` on a variable target. Claude Code's safety check holds those on a prompt
  nobody sees, and two verifiers and a builder hung for 3 to 7 hours at their cleanup step. Briefs say "leave your dir;
  the coordinator removes it". The coordinator checks transcript freshness (mtime of the output's target) on any agent
  silent for more than 30 minutes; a report on disk is harvestable without the agent.
- An identity check matches an immutable server-side id, never a display label. One round matched the pod verdict on
  `userId.username`, which the pod derives from a display name any installer picks; any pod member could post as
  the reviewer. Reviewer ids live in operator env, never in a file the PR can edit.
- A history scan covers every blob the range adds (`git rev-list --objects <base>..<head>`), not the merge result:
  `gh pr merge --merge` keeps intermediate commits, and gitleaks git mode skips binaries, images and allowlisted paths.
- Two paths that answer the same command must share ONE derivation of who the caller is. Four rounds running failed on
  the refusal path disagreeing with the handler (renewal, unauthorized/forbidden, run_stop, then a stray `caller` field
  read on types that have none). Fixing instances never converged. Ask for a single derivation and a differential test
  that runs both paths on mutated input from the start.
- A scanner cannot prove absence. Never let a clean scan WAIVE a review; a scan may only REFUSE. Four rounds went to making
  gitleaks read everything (images; NUL text and allowlisted paths; content sniffing; fragment cuts) so that "inert" PRs
  could land without the verdict. Each round found another blind spot, and each one was a bypass. The fix was
  subtraction: every land needs the verdict, and scans are defence in depth.
- Never let a scanner's own file-type detection decide what gets scanned. Land lost three rounds to gitleaks blind spots in
  turn (images, then NUL text and allowlisted paths, then content sniffing of PDF/ELF/zip magic). The machinery built to
  work around them (a strict text rule, a protected set for everything else, a fixed prefix so no magic matched) was deleted
  with the verdict-free path it served. The lesson stands: a scanner's blind spots are its own, so it only refuses.
- A leak check never prints what it matched: it reports file:line and class only, and a name-bearing path is withheld by
  position. Its test samples are synthetic and are grepped against the base tree before a round ships. A check that echoed
  a match would copy the secret into a log that masks only whole strings.
- When a subtraction deletes tests, a case is safe to delete only if every mutant it killed is in deleted code or is killed
  by another case. P08 r16 dropped the one case proving the range scan reads every commit, and the scan went untested.
- A test of a recipe runs it in the shape its caller does: same checkout, refs, cwd and env. P08 r17 tested a recipe in a shape its
  caller does not use, so a pass proved nothing about the real call.
- After a merge into a renaming branch, re-run the whole rename map over what the base side brought in, and re-run the
  mutant of every renamed test. OS1 r3 merged into a renaming branch, and what the base side
  brought in kept the old names.
- When each round finds new members of one mutant family, stop adding cases: pin the text or share the derivation. A test's
  reader is the caller's reader, never a copy. P08 r16-r18 added a case per round for one family and never converged.
- An output-hygiene rule lives in ONE rendering function, and its test combines the triggers (a withheld path together with
  a content hit). OS1 r4 showed the rule split across places, each tested on one trigger only.
