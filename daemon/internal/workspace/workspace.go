// Package workspace gives each agent run its own working directory under the daemon's workspace root. Without a repo it is a new
// empty directory; with one it is a git worktree of that repo on a branch named for the run. The root is the daemon's own 0700
// directory, never a repo, and nothing here removes anything that is not a run's directory directly under it.
//
// A worktree run's repo path lives in a <run>.repo sidecar beside its directory; cleanup reads that and never the directory,
// which the agent can write. A completed run's directory is removed. A failed run's is kept for inspection, marked by an empty <run>.kept file beside it,
// and only the newest KeepFailed of them stay. A daemon killed mid-run leaves unmarked directories; Sweep removes those at start.
package workspace

import (
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
)

// KeepFailed is how many failed runs' directories are kept: enough to look at the last few failures, few enough that an agent
// failing in a loop (each worktree a full checkout) cannot fill the disk.
const KeepFailed = 5

const (
	branchPrefix = "jellymoon/run-"
	keptSuffix   = ".kept"
	repoSuffix   = ".repo" // <run id>.repo beside the run dir holds the repo path of a worktree run; cleanup reads only this
)

// A run id is a directory name, so it can hold no path.
var nameOK = regexp.MustCompile(`^[A-Za-z0-9_-]{1,64}$`)

// Manager owns one workspace root.
type Manager struct {
	root string
	keep int
}

// New makes a manager for a root. Nothing is created until the first run needs it.
func New(root string) *Manager { return &Manager{root: root, keep: KeepFailed} }

// resolve makes the root (0700) and returns it with symlinks resolved, the form every later check compares against.
func (m *Manager) resolve() (string, error) {
	if err := os.MkdirAll(m.root, 0o700); err != nil {
		return "", err
	}
	if err := os.Chmod(m.root, 0o700); err != nil {
		return "", err
	}
	return filepath.EvalSymlinks(m.root)
}

// Prepare makes the run's directory and returns its path. With a repo (an absolute path) it is a worktree of the repo's HEAD on
// the branch jellymoon/run-<runID>; a run id is used once, so the branch cannot collide.
func (m *Manager) Prepare(runID, repo string) (string, error) {
	if !nameOK.MatchString(runID) {
		return "", fmt.Errorf("run id %q cannot name a directory", runID)
	}
	root, err := m.resolve()
	if err != nil {
		return "", err
	}
	dir := filepath.Join(root, runID)
	if repo == "" {
		return dir, os.Mkdir(dir, 0o700)
	}
	if !filepath.IsAbs(repo) {
		return "", fmt.Errorf("repo %q is not an absolute path", repo)
	}
	// The sidecar is written first, so even a half-made worktree is cleaned from the real repo.
	if err := os.WriteFile(dir+repoSuffix, []byte(repo), 0o600); err != nil {
		return "", err
	}
	if out, err := runGit(repo, true, "worktree", "add", "-q", "-b", branchPrefix+runID, dir, "HEAD"); err != nil {
		_ = m.remove(dir) // a half-made worktree
		return "", fmt.Errorf("worktree of %s: %v: %s", repo, err, out)
	}
	return dir, nil
}

// Finish ends a run's directory. A completed run's is removed. A failed run's stays, marked, and the oldest beyond keep go.
func (m *Manager) Finish(runID string, completed bool) error {
	root, err := m.resolve()
	if err != nil || !nameOK.MatchString(runID) {
		return errors.Join(err, errors.New("no such run directory"))
	}
	dir := filepath.Join(root, runID)
	if completed {
		return m.remove(dir)
	}
	if err := os.WriteFile(dir+keptSuffix, nil, 0o600); err != nil {
		return err
	}
	return m.prune(root)
}

// Sweep runs once at daemon start, before any run can exist: every run directory not marked kept belongs to a run that died with
// the previous daemon. Markers whose directory is gone go too, and the kept ones are trimmed to keep.
func (m *Manager) Sweep() error {
	if _, err := os.Lstat(m.root); errors.Is(err, os.ErrNotExist) {
		return nil
	}
	root, err := m.resolve()
	if err != nil {
		return err
	}
	entries, err := os.ReadDir(root)
	if err != nil {
		return err
	}
	var errs []error
	for _, e := range entries {
		name := e.Name()
		switch {
		case e.Type().IsDir() && !exists(filepath.Join(root, name+keptSuffix)):
			errs = append(errs, m.remove(filepath.Join(root, name)))
		case strings.HasSuffix(name, keptSuffix) && !exists(filepath.Join(root, strings.TrimSuffix(name, keptSuffix))):
			// the directory was lost: still drop its record and branch from the repo, then the marker
			if err := m.remove(filepath.Join(root, strings.TrimSuffix(name, keptSuffix))); err != nil {
				errs = append(errs, err)
			} else {
				errs = append(errs, os.Remove(filepath.Join(root, name)))
			}
		case strings.HasSuffix(name, repoSuffix) && !exists(filepath.Join(root, strings.TrimSuffix(name, repoSuffix))) && !exists(filepath.Join(root, strings.TrimSuffix(name, repoSuffix)+keptSuffix)):
			errs = append(errs, m.remove(filepath.Join(root, strings.TrimSuffix(name, repoSuffix))))
		}
	}
	return errors.Join(append(errs, m.prune(root))...)
}

func exists(p string) bool { _, err := os.Lstat(p); return err == nil }

// prune removes the oldest kept runs beyond keep.
func (m *Manager) prune(root string) error {
	marks, _ := filepath.Glob(filepath.Join(root, "*"+keptSuffix))
	age := func(p string) int64 {
		if fi, err := os.Lstat(p); err == nil {
			return fi.ModTime().UnixNano()
		}
		return 0
	}
	sort.Slice(marks, func(i, j int) bool {
		if ai, aj := age(marks[i]), age(marks[j]); ai != aj {
			return ai > aj
		}
		return marks[i] > marks[j]
	})
	var errs []error
	for i := m.keep; i < len(marks); i++ {
		// A marker goes only with its directory: when cleanup refused the directory, the run is still on the books.
		if err := m.remove(strings.TrimSuffix(marks[i], keptSuffix)); err != nil {
			errs = append(errs, err)
		} else {
			errs = append(errs, os.Remove(marks[i]))
		}
	}
	return errors.Join(errs...)
}

// remove deletes one run directory. It refuses unless path is, once cleaned, a valid run name directly inside the resolved root
// and a real directory there: a "..", a path elsewhere and a symlink (even one that points inside) are all refused. A worktree's
// record and branch are dropped with it. Only the run's own branch is ever deleted. The run is over, so what the directory
// holds, uncommitted work included, is disposable.
func (m *Manager) remove(path string) error {
	root, err := m.resolve()
	if err != nil {
		return err
	}
	path = filepath.Clean(path)
	parent, err := filepath.EvalSymlinks(filepath.Dir(path))
	name := filepath.Base(path)
	if err != nil || parent != root || !nameOK.MatchString(name) {
		return fmt.Errorf("refusing to remove %q: not a run directory directly under the workspace root", path)
	}
	dir := filepath.Join(root, name)
	fi, err := os.Lstat(dir)
	if err != nil && !errors.Is(err, os.ErrNotExist) {
		return err
	}
	if err == nil && !fi.IsDir() {
		return fmt.Errorf("refusing to remove %q: not a real directory", dir)
	}
	err = os.RemoveAll(dir)
	return errors.Join(err, m.forget(root, name))
}

// forget drops a worktree run's record and branch from the repo it was made from, and the sidecar. The repo comes from the
// sidecar only: the run directory is the agent's to write, so nothing in it (its .git file included) is read or run. No sidecar
// means no worktree run, and no git runs at all.
func (m *Manager) forget(root, name string) error {
	side := filepath.Join(root, name+repoSuffix)
	fi, err := os.Lstat(side)
	if err != nil || !fi.Mode().IsRegular() {
		return nil
	}
	raw, err := os.ReadFile(side)
	if repo := string(raw); err == nil && filepath.IsAbs(repo) {
		_, _ = runGit(repo, false, "worktree", "prune")
		_, _ = runGit(repo, false, "branch", "-D", branchPrefix+name)
	}
	return os.Remove(side)
}

// runGit runs git in repo with a small environment (the daemon's own may hold a token). With trusted false (every cleanup call)
// hooks and fsmonitor are off, so no program from the repo runs: core.hooksPath=/dev/null stops reference-transaction and every
// other hook, core.fsmonitor=false stops the fsmonitor command a repo's config can name. Prepare passes true: the user's own repo
// runs its own post-checkout hook there, as a normal checkout would.
func runGit(repo string, trusted bool, args ...string) (string, error) {
	pre := []string{"-C", repo}
	if !trusted {
		pre = append(pre, "-c", "core.hooksPath=/dev/null", "-c", "core.fsmonitor=false")
	}
	cmd := exec.Command("git", append(pre, args...)...)
	cmd.Env = []string{"PATH=" + os.Getenv("PATH"), "HOME=" + os.Getenv("HOME"), "GIT_TERMINAL_PROMPT=0"}
	out, err := cmd.CombinedOutput()
	return strings.TrimSpace(string(out)), err
}
