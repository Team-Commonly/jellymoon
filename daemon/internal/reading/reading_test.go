package reading

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

// A harness that never answers is stopped at the timeout, and there is no page. The e2e (TestF27) proves every other rule
// through the command; it cannot wait out the command's own three minutes.
func TestPageStopsACallThatTakesTooLong(t *testing.T) {
	bin := t.TempDir()
	if err := os.WriteFile(filepath.Join(bin, "claude"), []byte("#!/bin/sh\nexec sleep 30\n"), 0o700); err != nil {
		t.Fatal(err)
	}
	t.Setenv("PATH", bin+":/usr/bin:/bin")
	dir := filepath.Join(t.TempDir(), "reading")
	day := protocol.ReaderDay{Day: "2026-03-10", Turns: []protocol.ReaderTurn{{At: "09:00", Tool: protocol.HarnessClaudeCode, Project: "alpha", Said: "add a retry"}}}
	start := time.Now()
	page, err := Page(day, Options{Model: "sonnet", Dir: dir, Timeout: 200 * time.Millisecond})
	if err == nil || !strings.Contains(err.Error(), "took longer") || len(page.Wishes) != 0 || time.Since(start) > 10*time.Second {
		t.Errorf("after %s: page %+v, error %v; want the call stopped at its timeout", time.Since(start), page, err)
	}
	if left, _ := os.ReadDir(dir); len(left) != 0 {
		t.Errorf("the call's folder was left behind: %v", left)
	}
}
