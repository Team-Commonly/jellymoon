package reader

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// A line over the cap is read past and not kept, and the lines after it are still read. The cap is 64 MiB; here it is made
// small, so the test needs no giant file. Everything else the reader does is proved through the command (e2e TestF26).
func TestDaySkipsALineOverTheCap(t *testing.T) {
	defer func(max int) { lineMax = max }(lineMax)
	lineMax = 2000
	typed := func(at, text string) string {
		raw, _ := json.Marshal(map[string]any{"type": "user", "timestamp": at, "cwd": "/work/alpha", "origin": map[string]string{"kind": "human"}, "message": map[string]string{"content": text}})
		return string(raw)
	}
	dir := t.TempDir()
	lines := []string{typed("2026-03-10T10:00:00Z", "before the long line"), typed("2026-03-10T11:00:00Z", strings.Repeat("far too long ", 400)), typed("2026-03-10T12:00:00Z", "after the long line")}
	if err := os.MkdirAll(filepath.Join(dir, "-work-alpha"), 0o700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "-work-alpha", "s.jsonl"), []byte(strings.Join(lines, "\n")), 0o600); err != nil {
		t.Fatal(err)
	}
	day, err := Day(Options{Logs: dir, Day: time.Date(2026, 3, 10, 0, 0, 0, 0, time.UTC)})
	if err != nil || len(day.Turns) != 2 || day.Turns[0].Said != "before the long line" || day.Turns[1].Said != "after the long line" {
		t.Errorf("turns %+v, error %v; want the two short lines", day.Turns, err)
	}
}
