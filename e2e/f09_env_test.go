package e2e

import (
	"fmt"
	"slices"
	"strings"
	"syscall"
	"time"
)

// allowedAgentEnv is the daemon's allow-list (daemon/internal/daemon/run.go agentEnv), restated here on purpose: the suite
// pins the contract, not the implementation. PWD is not the daemon's to give: os/exec adds it to any child that has a working
// directory, and it holds only that directory.
var allowedAgentEnv = []string{"PATH", "HOME", "TMPDIR", "USER", "LANG", "LC_ALL", "TERM", "PWD"}

// scrubbedEnv: the daemon is restarted with computer.json already stored and a user token in its environment, and the suite's
// own environment holds none of the allowed names. The agent must still get only allowed names, which here means none.
func (f *f09) scrubbedEnv(record string) error {
	f.daemon.signal(syscall.SIGTERM)
	if _, ok := f.daemon.finish(15 * time.Second); !ok {
		return fmt.Errorf("the daemon did not exit within 15 s of SIGTERM")
	}
	f.harness(record)
	marks := f.marks()
	f.startDaemon("daemon-token", "JELLYMOON_USER_TOKEN="+f.owner.tok)
	if err := f.all("online", marks, 5*time.Second); err != nil {
		return err
	}
	f.assign("start the agent with a scrubbed environment")
	rec, err := f.recorded(record, func(r agentRec) bool { return set(r.Prompt) })
	if err != nil {
		return err
	}
	var extra []string
	for _, kv := range rec.Env {
		if name, _, _ := strings.Cut(kv, "="); !slices.Contains(allowedAgentEnv, name) {
			extra = append(extra, name)
		}
	}
	if len(extra) > 0 {
		return fmt.Errorf("the agent's environment holds names outside the allow-list %v: %v", allowedAgentEnv, extra)
	}
	return nil
}
