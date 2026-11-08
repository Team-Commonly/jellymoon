package workspace

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// hostile plants what an agent could: a repo of its own with a hook, a stale worktree record on the branch cleanup would delete,
// and a run directory whose .git file points at that record.
func hostile(t *testing.T, m *Manager, id string) (attacker, marker string) {
	t.Helper()
	attacker = newRepo(t)
	marker = filepath.Join(t.TempDir(), "hook-ran")
	hook := filepath.Join(attacker, ".git", "hooks", "reference-transaction")
	if err := os.WriteFile(hook, []byte("#!/bin/sh\ntouch '"+marker+"'\n"), 0o755); err != nil {
		t.Fatal(err)
	}
	gone := filepath.Join(t.TempDir(), "gone")
	git(t, attacker, "worktree", "add", "-q", "-b", branchPrefix+id, gone)
	if err := os.RemoveAll(gone); err != nil {
		t.Fatal(err)
	}
	_ = os.Remove(marker) // the setup's own ref updates may have fired the hook
	dir, err := m.Prepare(id, "")
	if err != nil {
		t.Fatal(err)
	}
	link := "gitdir: " + filepath.Join(attacker, ".git", "worktrees", "gone") + "\n"
	if err := os.WriteFile(filepath.Join(dir, ".git"), []byte(link), 0o644); err != nil {
		t.Fatal(err)
	}
	return attacker, marker
}

func untouched(t *testing.T, how, id, attacker, marker, dir string) {
	t.Helper()
	if exists(marker) {
		t.Errorf("%s: the attacker's hook ran as the daemon", how)
	}
	if !strings.Contains(git(t, attacker, "branch", "--list", branchPrefix+id), id) {
		t.Errorf("%s: the attacker's branch was deleted", how)
	}
	if exists(dir) {
		t.Errorf("%s: the run directory is still there", how)
	}
}

// The agent can rewrite its directory's .git file. Cleanup must not ask git about the directory, so a .git pointing at another
// repo changes nothing there: no hook runs and no branch goes.
func TestHostileDotGitIsNeverTrusted(t *testing.T) {
	m := newManager(t)
	a, marker := hostile(t, m, "run_fin")
	if err := m.Finish("run_fin", true); err != nil {
		t.Fatal(err)
	}
	untouched(t, "finish", "run_fin", a, marker, filepath.Join(m.root, "run_fin"))

	a, marker = hostile(t, m, "run_swp")
	if err := New(m.root).Sweep(); err != nil {
		t.Fatal(err)
	}
	untouched(t, "sweep", "run_swp", a, marker, filepath.Join(m.root, "run_swp"))

	m.keep = 0
	a, marker = hostile(t, m, "run_prn")
	if err := m.Finish("run_prn", false); err != nil {
		t.Fatal(err)
	}
	untouched(t, "prune", "run_prn", a, marker, filepath.Join(m.root, "run_prn"))
}

// A worktree directory removed by hand, or with its .git broken, still leaves no record or branch in the real repo.
func TestLostWorktreeDirStillCleansTheRepo(t *testing.T) {
	m, repo := newManager(t), newRepo(t)
	byHand, err := m.Prepare("run_hand", repo)
	if err != nil {
		t.Fatal(err)
	}
	broken, err := m.Prepare("run_brk", repo)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.RemoveAll(byHand); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(broken, ".git"), []byte("garbage"), 0o644); err != nil {
		t.Fatal(err)
	}
	for _, id := range []string{"run_hand", "run_brk"} {
		if err := m.Finish(id, true); err != nil {
			t.Fatal(err)
		}
	}
	if l := git(t, repo, "worktree", "list"); strings.Contains(l, "run_") || git(t, repo, "branch", "--list", "jellymoon/*") != "" {
		t.Errorf("the repo still knows the runs:\n%s\n%s", l, git(t, repo, "branch", "--list"))
	}
	if exists(broken) {
		t.Errorf("the broken directory is still there")
	}
}

// Cleanup runs git with hooks off even in the real repo: a hook there (a reference-transaction one fires on the branch delete)
// does not run, though Prepare's own checkout is allowed to run the repo's hooks.
func TestCleanupRunsNoHooksInTheRealRepo(t *testing.T) {
	m, repo := newManager(t), newRepo(t)
	marker := filepath.Join(t.TempDir(), "hook-ran")
	hook := filepath.Join(repo, ".git", "hooks", "reference-transaction")
	if err := os.WriteFile(hook, []byte("#!/bin/sh\ntouch '"+marker+"'\n"), 0o755); err != nil {
		t.Fatal(err)
	}
	if _, err := m.Prepare("run_h", repo); err != nil {
		t.Fatal(err)
	}
	if !exists(marker) {
		t.Fatalf("setup: Prepare did not run the repo's own hook")
	}
	_ = os.Remove(marker)
	if err := m.Finish("run_h", true); err != nil {
		t.Fatal(err)
	}
	if exists(marker) {
		t.Errorf("cleanup ran a hook of the repo")
	}
	if git(t, repo, "branch", "--list", "jellymoon/*") != "" {
		t.Errorf("the run's branch is still there")
	}
}

// When the cleanup refuses a directory (here, one swapped for a symlink), the kept marker stays: the run is still on the books.
func TestPruneKeepsTheMarkerOfARefusedDir(t *testing.T) {
	m := newManager(t)
	m.keep = 1
	dir, _ := m.Prepare("run_a", "")
	if err := m.Finish("run_a", false); err != nil {
		t.Fatal(err)
	}
	if err := os.Remove(dir); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(t.TempDir(), dir); err != nil {
		t.Fatal(err)
	}
	_, _ = m.Prepare("run_b", "")
	_ = m.Finish("run_b", false)
	if !exists(dir + keptSuffix) {
		t.Errorf("the marker of a directory cleanup refused was deleted")
	}
}
