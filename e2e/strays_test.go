package e2e

import (
	"fmt"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
	"time"
)

// noStrays fails the test if a process started from its temp dirs (the built daemon, the fake agent) is still alive when it
// ends. t.TempDir hands out numbered dirs under one per-test base, so the base names everything the test built. par registers
// it first, so it runs after every other cleanup. A stray is killed here so one failure does not pile up processes.
func noStrays(t *testing.T) {
	base := filepath.Dir(t.TempDir()) + "/"
	t.Cleanup(func() {
		var left []string
		for end := time.Now().Add(3 * time.Second); ; time.Sleep(50 * time.Millisecond) {
			left = left[:0]
			out, _ := exec.Command("ps", "-axo", "pid=,command=").Output()
			for _, line := range strings.Split(string(out), "\n") {
				if f := strings.Fields(line); len(f) > 1 && strings.HasPrefix(f[1], base) {
					left = append(left, line)
				}
			}
			if len(left) == 0 || time.Now().After(end) {
				break
			}
		}
		for _, line := range left {
			var pid int
			if n, _ := fmt.Sscan(line, &pid); n == 1 {
				_ = syscall.Kill(pid, syscall.SIGKILL)
			}
			t.Errorf("a process outlived the test: %s", strings.TrimSpace(line))
		}
	})
}
