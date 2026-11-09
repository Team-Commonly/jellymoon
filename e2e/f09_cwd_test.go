package e2e

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"time"
)

// workspaceDir: the agent's cwd is a new directory named for its run under the daemon's workspace root (the config dir's sibling,
// restated here on purpose: the suite pins the contract, not the code), and the directory is gone once the run completed.
func (f *f09) workspaceDir(record string) error {
	rec, err := f.recorded(record, func(r agentRec) bool { return set(r.SessionNew) })
	if err != nil {
		return err
	}
	var np struct{ Cwd string }
	_ = json.Unmarshal(rec.SessionNew, &np)
	root, err := filepath.EvalSymlinks(f.cfg + "-workspaces")
	if err != nil {
		return fmt.Errorf("the workspace root %s-workspaces does not exist: %v", f.cfg, err)
	}
	if want := filepath.Join(root, f.run.RunID); np.Cwd != want {
		return fmt.Errorf("session/new cwd = %q, want %q", np.Cwd, want)
	}
	for end := time.Now().Add(5 * time.Second); ; time.Sleep(25 * time.Millisecond) {
		if _, err := os.Lstat(np.Cwd); err != nil {
			return nil
		}
		if time.Now().After(end) {
			return fmt.Errorf("the run's directory %s is still there 5 s after the run completed", np.Cwd)
		}
	}
}
