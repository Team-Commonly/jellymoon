GOAL         one sentence, the outcome, executable by a stranger with no chat access
SCOPE        paths this unit may write; paths it may not; its exclusive worktree and branch.
             Say whether package.json is writable: a new script is dead to knip unless a package.json
             script or an import reaches it.
CONTEXT      pointers to repo files and PRs; upstream reports pasted in full when this unit depends on them
GOVERNING ADRs  each Accepted ADR clause the brief relies on or touches, quoted verbatim with file:line (`.loop/gates.md`)
ACCEPTANCE   checkable criteria, one per line
VERIFY       exact commands, plus known gotchas. Every command runs through the clean-shell runner
             (empty HOME, system PATH) so a hidden machine dependency fails instead of passing here.
TIMEBOX      rough cap; on expiry, commit what is green, report partial findings and stop
FORBIDDEN    no push, merge, rebase, reset, force-push or PR operations; no edits outside SCOPE; no getting past a
             check by any route other than fixing what it found (indirection, renaming, a wrapper, a suppression
             comment). If a check blocks you and its config is outside SCOPE, stop and report it. (Earned: a constant
             naming a binary hid that binary from knip.) Plus unit bans.
             Never `rm -rf` or `chmod -R` on a variable target (`rm -rf $D/x`, `rm -rf *`). Claude Code's safety check
             holds it on a permission prompt nobody sees, and the agent hangs: two verifiers and a builder once sat
             hours at cleanup. Use a literal absolute path or `"${D:?}"/x`, or leave the removal to the coordinator;
             a verifier never cleans up, it leaves its dir. Write only inside SCOPE and your scratch dir, never a
             shared temp dir.
             Call executables by literal absolute path, never `$VAR/tool`, and write scripts with the Write tool, not a
             heredoc that expands shell variables: both are held on the same unseen prompt (a builder once hung 30
             minutes on `$S/run-tool`).
REPORT       status, branch, head SHA, what you ran and its output, lines added/removed (git diff --shortstat),
             what you deleted or why nothing, deviations, where you got stuck and for how long, follow-ups
STANDING     `.loop/preferences.md` (read it in full first)
             Runner gotcha: the clean-shell runner passes the command through bash -c "$*"; for nested quotes or
             sed -i, write a script file in the scratch dir and run that.
