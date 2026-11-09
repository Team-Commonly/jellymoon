package workspace

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

func git(t *testing.T, dir string, args ...string) string {
	t.Helper()
	cmd := exec.Command("git", append([]string{"-C", dir, "-c", "user.name=t", "-c", "user.email=t@example.invalid"}, args...)...)
	cmd.Env = []string{"PATH=" + os.Getenv("PATH"), "HOME=" + t.TempDir(), "GIT_CONFIG_GLOBAL=/dev/null", "GIT_CONFIG_SYSTEM=/dev/null"}
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("git %v: %v\n%s", args, err, out)
	}
	return strings.TrimSpace(string(out))
}

func newRepo(t *testing.T) string {
	t.Helper()
	repo := t.TempDir()
	git(t, repo, "init", "-q", "-b", "main")
	if err := os.WriteFile(filepath.Join(repo, "hello.txt"), []byte("hi"), 0o644); err != nil {
		t.Fatal(err)
	}
	git(t, repo, "add", ".")
	git(t, repo, "commit", "-q", "-m", "first")
	return repo
}

func newManager(t *testing.T) *Manager {
	t.Helper()
	return New(filepath.Join(t.TempDir(), "workspaces"))
}

// A run without a repo gets a new, empty directory directly under the root, mode 0700.
func TestPlainRunGetsAFreshDirUnderTheRoot(t *testing.T) {
	m := newManager(t)
	dir, err := m.Prepare("run_a", "")
	if err != nil {
		t.Fatal(err)
	}
	if filepath.Base(dir) != "run_a" || filepath.Dir(dir) != mustResolve(t, m.root) {
		t.Errorf("dir = %s, want run_a directly under %s", dir, m.root)
	}
	if es, _ := os.ReadDir(dir); len(es) != 0 {
		t.Errorf("the directory is not empty: %v", es)
	}
	if fi, _ := os.Stat(dir); fi.Mode().Perm() != 0o700 {
		t.Errorf("mode = %v, want 0700", fi.Mode().Perm())
	}
	if _, err := m.Prepare("run_a", ""); err == nil {
		t.Errorf("a second Prepare of the same run id reused the directory")
	}
	if _, err := m.Prepare("../run_b", ""); err == nil {
		t.Errorf("a run id with a path in it was accepted")
	}
}

func mustResolve(t *testing.T, p string) string {
	t.Helper()
	r, err := filepath.EvalSymlinks(p)
	if err != nil {
		t.Fatal(err)
	}
	return r
}

// With a repo, the directory is a worktree of it on a branch named for the run, so two runs never share a branch.
func TestRepoRunGetsAWorktreeOnAPerRunBranch(t *testing.T) {
	m, repo := newManager(t), newRepo(t)
	a, err := m.Prepare("run_a", repo)
	if err != nil {
		t.Fatal(err)
	}
	b, err := m.Prepare("run_b", repo)
	if err != nil {
		t.Fatal(err)
	}
	if raw, err := os.ReadFile(filepath.Join(a, "hello.txt")); err != nil || string(raw) != "hi" {
		t.Errorf("the worktree does not hold the repo's files: %q, %v", raw, err)
	}
	if ba, bb := git(t, a, "rev-parse", "--abbrev-ref", "HEAD"), git(t, b, "rev-parse", "--abbrev-ref", "HEAD"); ba != "jellymoon/run-run_a" || bb != "jellymoon/run-run_b" {
		t.Errorf("branches = %q and %q, want one per run", ba, bb)
	}
	if got := strings.Count(git(t, repo, "worktree", "list"), "\n") + 1; got != 3 {
		t.Errorf("the repo lists %d worktrees, want the checkout plus two", got)
	}
}

// A completed run leaves nothing behind: not the directory, not the worktree record, not the branch.
func TestCompletedRunLeavesNothing(t *testing.T) {
	m, repo := newManager(t), newRepo(t)
	plain, _ := m.Prepare("run_p", "")
	tree, err := m.Prepare("run_w", repo)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(tree, "scratch"), []byte("x"), 0o644); err != nil {
		t.Fatal(err)
	}
	for _, id := range []string{"run_p", "run_w"} {
		if err := m.Finish(id, true); err != nil {
			t.Fatal(err)
		}
	}
	if exists(plain) || exists(tree) {
		t.Errorf("a directory is left after a completed run: %v %v", exists(plain), exists(tree))
	}
	if strings.Contains(git(t, repo, "worktree", "list"), "run_w") || git(t, repo, "branch", "--list", "jellymoon/*") != "" {
		t.Errorf("the repo still knows the run:\n%s\n%s", git(t, repo, "worktree", "list"), git(t, repo, "branch", "--list"))
	}
}

// A failed run's directory stays for inspection, but only the newest KeepFailed (K) of them: a loop of failing runs must not fill the disk.
func TestFailedRunsAreKeptUpToK(t *testing.T) {
	m := newManager(t)
	m.keep = 2
	var dirs []string
	for _, id := range []string{"run_1", "run_2", "run_3", "run_4"} {
		d, err := m.Prepare(id, "")
		if err != nil {
			t.Fatal(err)
		}
		dirs = append(dirs, d)
		if err := m.Finish(id, false); err != nil {
			t.Fatal(err)
		}
	}
	for i, want := range []bool{false, false, true, true} {
		if got := exists(dirs[i]); got != want {
			t.Errorf("run_%d kept = %v, want %v", i+1, got, want)
		}
	}
	if KeepFailed != 5 {
		t.Errorf("KeepFailed = %d; the docs name 5", KeepFailed)
	}
}

// A daemon killed mid-run leaves its directories behind. The next start removes them (worktree records and branches too) and
// keeps what failed runs left on purpose.
func TestSweepRemovesStaleRunsAndKeepsFailedOnes(t *testing.T) {
	m, repo := newManager(t), newRepo(t)
	stale, _ := m.Prepare("run_stale", "")
	staleTree, err := m.Prepare("run_staletree", repo)
	if err != nil {
		t.Fatal(err)
	}
	kept, _ := m.Prepare("run_kept", "")
	if err := m.Finish("run_kept", false); err != nil {
		t.Fatal(err)
	}
	if err := New(m.root).Sweep(); err != nil {
		t.Fatal(err)
	}
	if exists(stale) || exists(staleTree) {
		t.Errorf("a stale run directory survived the sweep: %v %v", exists(stale), exists(staleTree))
	}
	if !exists(kept) {
		t.Errorf("the sweep removed a failed run's kept directory")
	}
	if strings.Contains(git(t, repo, "worktree", "list"), "run_staletree") || git(t, repo, "branch", "--list", "jellymoon/*") != "" {
		t.Errorf("the repo still knows the stale run:\n%s", git(t, repo, "worktree", "list"))
	}
	if err := New(filepath.Join(t.TempDir(), "never-made")).Sweep(); err != nil {
		t.Errorf("sweeping a root that does not exist: %v", err)
	}
}

// Cleanup only ever removes a run's own directory directly under the root. A "..", an absolute path elsewhere or a planted
// symlink must leave the target alone.
func TestCleanupRefusesAnythingOutsideTheRoot(t *testing.T) {
	m := newManager(t)
	if _, err := m.Prepare("run_real", ""); err != nil {
		t.Fatal(err)
	}
	outside := t.TempDir()
	victim := filepath.Join(outside, "precious")
	if err := os.WriteFile(victim, []byte("keep"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(outside, filepath.Join(m.root, "run_link")); err != nil {
		t.Fatal(err)
	}
	for _, p := range []string{outside, filepath.Join(m.root, "..", filepath.Base(outside)), filepath.Join(m.root, "run_link"), filepath.Join(m.root, "run_real", "..", "..")} {
		if err := m.remove(p); err == nil {
			t.Errorf("remove(%s) was not refused", p)
		}
	}
	// The same through the public paths: a run whose directory was replaced by a symlink is not followed.
	_ = m.Finish("run_link", true)
	_ = m.Sweep()
	if !exists(victim) {
		t.Fatalf("a cleanup removed a file outside the workspace root")
	}
	// A link inside a run directory is removed as a link; its target stays.
	inner, _ := m.Prepare("run_inner", "")
	if err := os.Symlink(outside, filepath.Join(inner, "out")); err != nil {
		t.Fatal(err)
	}
	if err := m.Finish("run_inner", true); err != nil || !exists(victim) || exists(inner) {
		t.Errorf("finish run_inner: err=%v victim kept=%v dir gone=%v", err, exists(victim), !exists(inner))
	}
}
