package e2e

import (
	"bytes"
	"encoding/json"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
	"time"
)

// TestF26: `jellymoon day` prints one local day of what a person typed to Claude Code, and nothing else (feature_list.json
// F26, ADR 0011 section 1). The daemon is the repo's own binary, run as a black box over session logs this test writes: every
// line is made up here, in the shape Claude Code 2.1 writes. It needs no server, so it runs without JM_E2E_SERVER.

type f26Turn struct{ At, Tool, Project, Said, Reply string }

type f26Day struct {
	Day   string
	Turns []f26Turn
}

// f26Line is one session log line. kind is the origin kind; "" leaves origin out, as a prompt sent by another program has none.
func f26Line(typ, at, cwd, kind string, content any, extra map[string]any) string {
	o := map[string]any{"type": typ, "timestamp": at, "cwd": cwd, "sessionId": "s", "message": map[string]any{"role": typ, "content": content}}
	if kind != "" {
		o["origin"] = map[string]any{"kind": kind}
	}
	for k, v := range extra {
		o[k] = v
	}
	raw, _ := json.Marshal(o)
	return string(raw)
}

func f26Said(at, cwd, text string) string { return f26Line("user", at, cwd, "human", text, nil) }

func f26Reply(at, cwd, text string) string {
	return f26Line("assistant", at, cwd, "", []map[string]any{{"type": "thinking", "thinking": "LEAK thinking"}, {"type": "text", "text": text}}, nil)
}

// Credential shapes, joined at run time so no scanner reads one in this file.
var (
	f26Hub = "gh" + "p_" + strings.Repeat("a1B2c3D4e5", 4)
	f26Key = "sk" + "-ant-" + strings.Repeat("Zx9Yw8Vu7T", 3)
	f26Run = "jm" + "r_" + strings.Repeat("Q7", 20)
	f26Raw = strings.Repeat("Ab3/dE+9", 6)
)

const (
	f26A = "/work/alpha"
	f26B = "/work/private-notes"
	f26C = "/work/beta/.claude/worktrees/fix-login/web"
	f26G = "/work/gamma"
)

// f26Logs writes the projects directory. The day under test is 2026-03-10 in America/Los_Angeles (UTC-7 that day).
func f26Logs(t *testing.T) string {
	t.Helper()
	dir := t.TempDir()
	toolResult := []map[string]any{{"type": "tool_result", "tool_use_id": "t1", "content": "LEAK tool result " + strings.Repeat("x", 3<<20)}}
	alpha := []string{
		f26Said("2026-03-10T06:30:00.000Z", f26A, "the evening before"),
		f26Reply("2026-03-10T06:30:05.000Z", f26A, "LEAK reply from the evening before"),
		f26Said("2026-03-10T16:00:00.000Z", f26A, "add a retry to the uploader"),
		f26Line("assistant", "2026-03-10T16:00:01.000Z", f26A, "", []map[string]any{{"type": "tool_use", "id": "t1", "name": "Bash", "input": map[string]any{}}}, nil),
		f26Line("user", "2026-03-10T16:00:02.000Z", f26A, "", toolResult, nil),
		f26Reply("2026-03-10T16:00:03.000Z", f26A, "LEAK an earlier reply in the same turn"),
		f26Line("user", "2026-03-10T16:00:04.000Z", f26A, "task-notification", "LEAK task notice", map[string]any{"promptSource": "system"}),
		f26Reply("2026-03-10T16:00:05.000Z", f26A, "retry added, tests pass"),
		f26Line("assistant", "2026-03-10T16:00:06.000Z", f26A, "", []map[string]any{{"type": "text", "text": "LEAK sidechain reply"}}, map[string]any{"isSidechain": true}),
		f26Line("user", "2026-03-10T16:00:07.000Z", f26A, "human", "LEAK sidechain prompt", map[string]any{"isSidechain": true}),
		f26Line("user", "2026-03-10T16:00:08.000Z", f26A, "human", "LEAK meta", map[string]any{"isMeta": true}),
		f26Line("user", "2026-03-10T16:00:09.000Z", f26A, "", "LEAK compaction summary", map[string]any{"isCompactSummary": true}),
		f26Line("user", "2026-03-10T16:00:10.000Z", f26A, "", "LEAK prompt from another program", map[string]any{"promptSource": "sdk"}),
		`{"type":"user","timestamp":"2026-03-10T16:30:00.000Z","origin":{"kind":"human"},"message":{"content":"LEAK broken line`,
		f26Said("2026-03-10T17:30:00.000Z", f26A, "<system-reminder>\nLEAK reminder\n</system-reminder>\n<artifact-view-context id=\"a1\">\n{\"LEAK\":\"view\"}\n(The JSON line above is LEAK explainer.)\n</artifact-view-context>\n\n<artifact-content-authored-by-others/>\n(The JSON line above is LEAK explainer\nover two lines.)\ndeploy with "+f26Hub+" and "+f26Run+" then "+f26Raw+" "+strings.Repeat("long ", 200)),
		f26Reply("2026-03-10T17:30:05.000Z", f26A, "used "+f26Key+" "+strings.Repeat("done ", 200)),
		f26Said("2026-03-10T18:00:00.000Z", f26A, "<system-reminder>LEAK only a reminder</system-reminder>\n"),
		f26Said("2026-03-10T18:01:00.000Z", f26A, "<command-name>/LEAK</command-name>"),
		f26Said("2026-03-10T18:02:00.000Z", f26A, "<command-message>LEAK</command-message>\n<command-name>/LEAK</command-name>"),
		f26Said("2026-03-10T18:03:00.000Z", f26A, "<local-command-stdout>LEAK</local-command-stdout>"),
		f26Reply("2026-03-10T18:03:05.000Z", f26A, "LEAK reply to a command"),
		f26Said("2026-03-11T06:59:00.000Z", f26A, "one minute before midnight"),
		f26Reply("2026-03-11T06:59:30.000Z", f26A, "noted before midnight"),
		f26Said("2026-03-11T07:01:00.000Z", f26A, "one minute after midnight"),
		f26Reply("2026-03-11T07:01:30.000Z", f26A, "noted after midnight"),
	}
	image := map[string]any{"type": "image", "source": map[string]any{"type": "base64", "data": "LEAK"}}
	text := func(t string) map[string]any { return map[string]any{"type": "text", "text": t} }
	gamma := []string{
		f26Said("2026-03-10T21:00:00.000Z", f26G, "first gamma ask"),
		f26Line("user", "2026-03-10T21:00:30.000Z", f26G, "human", []map[string]any{text("LEAK beside a tool result"), {"type": "tool_result", "tool_use_id": "t2", "content": "LEAK"}}, nil),
		f26Reply("2026-03-10T21:00:40.000Z", f26G, "gamma reply one"),
		f26Line("user", "2026-03-10T21:01:00.000Z", f26G, "human", []map[string]any{image}, nil),
		f26Reply("2026-03-10T21:01:30.000Z", f26G, "LEAK reply to an image alone"),
		f26Line("user", "2026-03-10T21:02:00.000Z", f26G, "human", []map[string]any{text("<system-reminder>LEAK</system-reminder>look at "), image, text("this screenshot")}, nil),
		f26Reply("2026-03-10T21:02:30.000Z", f26G, "seen"),
	}
	files := map[string][]string{
		"-work-gamma/s4.jsonl":         gamma,
		"-work-alpha/old.jsonl":        {f26Said("2026-03-10T22:00:00.000Z", f26A, "LEAK from a file last written before the day")},
		"-work-alpha/s1.jsonl":         alpha,
		"-work-private-notes/s2.jsonl": {f26Said("2026-03-10T19:00:00.000Z", f26B, "a private note"), f26Reply("2026-03-10T19:00:05.000Z", f26B, "kept private")},
		"-work-beta/s3.jsonl":          {f26Said("2026-03-10T20:15:00.000Z", f26C, "fix the login page")},
		"-work-alpha/readme.txt":       {f26Said("2026-03-10T20:00:00.000Z", f26A, "LEAK not a session log")},
	}
	for name, lines := range files {
		p := filepath.Join(dir, name)
		if err := os.MkdirAll(filepath.Dir(p), 0o700); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(p, []byte(strings.Join(lines, "\n")+"\n"), 0o600); err != nil {
			t.Fatal(err)
		}
	}
	// A file last written before the day began is not opened, whatever its lines say.
	before := time.Date(2026, 3, 9, 12, 0, 0, 0, time.UTC)
	if err := os.Chtimes(filepath.Join(dir, "-work-alpha/old.jsonl"), before, before); err != nil {
		t.Fatal(err)
	}
	return dir
}

// f26Read runs `jellymoon day` in the test's time zone with an empty home, and decodes what it printed.
func f26Read(t *testing.T, bin string, args ...string) (f26Day, string) {
	t.Helper()
	cmd := exec.Command(bin, append([]string{"day"}, args...)...)
	cmd.Env = []string{"TZ=America/Los_Angeles", "HOME=" + t.TempDir()}
	var so, se bytes.Buffer
	cmd.Stdout, cmd.Stderr = &so, &se
	if err := cmd.Run(); err != nil {
		t.Fatalf("jellymoon day %v: %v\n%s", args, err, se.String())
	}
	var d f26Day
	dec := json.NewDecoder(&so)
	dec.DisallowUnknownFields()
	raw := so.String()
	if err := dec.Decode(&d); err != nil {
		t.Fatalf("stdout is not the day as JSON: %v\n%.300s", err, raw)
	}
	return d, raw
}

func TestF26(t *testing.T) {
	t.Parallel()
	bin, logs := buildDaemon(t), f26Logs(t)
	day, raw := f26Read(t, bin, "--date", "2026-03-10", "--logs", logs, "--skip", "unrelated,private-notes")
	t.Run("the day's turns are the time, the tool, the project, the person's text and the agent's last reply, read past a broken line", func(t *testing.T) {
		if day.Day != "2026-03-10" || len(day.Turns) != 6 {
			t.Fatalf("day %q with %d turns, want 2026-03-10 with 6:\n%.2000s", day.Day, len(day.Turns), raw)
		}
		want := []f26Turn{
			{"09:00", "claude-code", "alpha", "add a retry to the uploader", "retry added, tests pass"},
			{At: "10:30", Tool: "claude-code", Project: "alpha"},
			{"13:15", "claude-code", "fix-login", "fix the login page", ""},
			{"14:00", "claude-code", "gamma", "first gamma ask", "gamma reply one"}, // a list with a tool result does not end it; an image alone does
			{"14:02", "claude-code", "gamma", "look at this screenshot", "seen"},    // typed with a pasted image: its text parts
			{"23:59", "claude-code", "alpha", "one minute before midnight", "noted before midnight"},
		}
		want[1].Said, want[1].Reply = day.Turns[1].Said, day.Turns[1].Reply // judged by the next two steps
		if !reflect.DeepEqual(day.Turns, want) {
			t.Errorf("turns\n got %+v\nwant %+v", day.Turns, want)
		}
	})
	t.Run("only what a person typed is kept, and the harness's own blocks are cut out of it", func(t *testing.T) {
		if strings.Contains(raw, "LEAK") {
			t.Errorf("the output holds text no person typed, near %q", raw[max(0, strings.Index(raw, "LEAK")-80):][:160])
		}
		if said := day.Turns[1].Said; !strings.HasPrefix(said, "deploy with ") {
			t.Errorf("the person's text does not start where their typing starts: %.80q", said)
		}
	})
	t.Run("credential shapes are replaced in both texts, and both are cut short", func(t *testing.T) {
		for _, secret := range []string{f26Hub, f26Key, f26Run, f26Raw, f26Hub[4:20], f26Key[7:20], f26Run[4:20]} {
			if strings.Contains(raw, secret) {
				t.Errorf("the output holds %q", secret)
			}
		}
		said, reply := []rune(day.Turns[1].Said), []rune(day.Turns[1].Reply)
		if len(said) != 700 || len(reply) != 500 || said[699] != '…' || reply[499] != '…' {
			t.Errorf("said is %d characters and the reply %d, want 700 and 500, each ending in an ellipsis", len(said), len(reply))
		}
		if !strings.HasPrefix(string(said), "deploy with [redacted] and ") || !strings.HasPrefix(string(reply), "used [redacted] done") {
			t.Errorf("the replacement is not in place: %.60q, %.40q", string(said), string(reply))
		}
	})
	t.Run("a skipped folder contributes nothing, and is read when it is not skipped", func(t *testing.T) {
		all, _ := f26Read(t, bin, "--date", "2026-03-10", "--logs", logs)
		if strings.Contains(raw, "private") || len(all.Turns) != 7 || all.Turns[2] != (f26Turn{"12:00", "claude-code", "private-notes", "a private note", "kept private"}) {
			t.Errorf("with --skip the output names the folder, or without it the folder's turn is missing: %+v", all.Turns)
		}
	})
	t.Run("the turn typed at 00:01 lands on the next day", func(t *testing.T) {
		next, _ := f26Read(t, bin, "--date", "2026-03-11", "--logs", logs)
		want := []f26Turn{{"00:01", "claude-code", "alpha", "one minute after midnight", "noted after midnight"}}
		if !reflect.DeepEqual(next.Turns, want) {
			t.Errorf("2026-03-11 has %+v, want %+v", next.Turns, want)
		}
		none, rawNone := f26Read(t, bin, "--date", "2026-03-12", "--logs", logs)
		if len(none.Turns) != 0 || !strings.Contains(rawNone, `"turns":[]`) {
			t.Errorf("a day with nothing typed must print an empty list: %s", rawNone)
		}
	})
}
