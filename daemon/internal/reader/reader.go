// Package reader collects one local day of what a person typed to Claude Code from the harness's session logs, with the agent's
// last text reply to each turn (ADR 0011, section 1). A session log is untrusted and mostly not the person's words: only a line
// the log itself marks as typed by a person starts a turn. Nothing leaves this package before redact.Credentials has seen it.
package reader

import (
	"bufio"
	"bytes"
	"encoding/json"
	"os"
	"path"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"time"

	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
	"github.com/Team-Commonly/jellymoon/daemon/internal/redact"
)

// The lengths said and reply are cut to, in characters (schema/journal.json).
const (
	saidMax    = 700
	replyMax   = 500
	projectMax = 255
)

// Options is one read: the Claude Code projects directory, the day (its Location is the person's time zone) and the names
// of the folders the person switched off.
type Options struct {
	Logs string
	Day  time.Time
	Skip []string
}

// line is the part of a session log line the reader looks at (Claude Code 2.1).
type line struct {
	Type             string          `json:"type"`
	Timestamp        time.Time       `json:"timestamp"`
	Cwd              string          `json:"cwd"`
	IsMeta           bool            `json:"isMeta"`
	IsSidechain      bool            `json:"isSidechain"`
	IsCompactSummary bool            `json:"isCompactSummary"`
	Origin           *origin         `json:"origin"`
	Message          json.RawMessage `json:"message"`
}

type origin struct {
	Kind string `json:"kind"`
}

type part struct {
	Type string `json:"type"`
	Text string `json:"text"`
}

// turn is a kept turn before it is printed: when it was typed orders the day.
type turn struct {
	at time.Time
	protocol.ReaderTurn
}

// harnessBlocks is what the harness adds inside a person's prompt: reminders, the view context of an open artifact and the
// paragraph that explains it.
var harnessBlocks = regexp.MustCompile(`(?s)<system-reminder>.*?</system-reminder>|<artifact-view-context.*?</artifact-view-context>|<artifact-content-authored-by-others\s*/>|\(The JSON line above[^)]*\)`)

var originKey, assistantWord = []byte(`"origin"`), []byte(`"assistant"`)

var worktree = regexp.MustCompile(`([^/]+)/\.claude/worktrees/`)

// Day reads every session log under o.Logs and returns the turns typed on o.Day, oldest first.
func Day(o Options) (protocol.ReaderDay, error) {
	out := protocol.ReaderDay{Day: o.Day.Format(time.DateOnly), Turns: []protocol.ReaderTurn{}}
	if _, err := os.Stat(o.Logs); err != nil {
		return out, err
	}
	files, err := filepath.Glob(filepath.Join(o.Logs, "*", "*.jsonl"))
	if err != nil {
		return out, err
	}
	end := o.Day.AddDate(0, 0, 1)
	var turns []turn
	for _, f := range files {
		// A file last written before the day began holds none of it.
		if fi, err := os.Stat(f); err != nil || fi.ModTime().Before(o.Day) {
			continue
		}
		for _, t := range session(f, o.Skip) {
			if !t.at.Before(o.Day) && t.at.Before(end) {
				t.At = t.at.In(o.Day.Location()).Format("15:04")
				turns = append(turns, t)
			}
		}
	}
	sort.SliceStable(turns, func(i, j int) bool { return turns[i].at.Before(turns[j].at) })
	for _, t := range turns {
		out.Turns = append(out.Turns, t.ReaderTurn)
	}
	return out, nil
}

// session returns one log's turns, on any day. A line that is not valid JSON is skipped. Lines run to megabytes (a tool result
// is one line), so they are read whole, with no fixed buffer; a file that cannot be read gives what was read before the error.
func session(file string, skip []string) []turn {
	f, err := os.Open(file)
	if err != nil {
		return nil
	}
	defer f.Close()
	var turns []turn
	open := false // whether the last turn can still take a reply
	for r := bufio.NewReaderSize(f, 1<<16); ; {
		raw, err := r.ReadBytes('\n')
		var l line
		// Most of a log is tool results, megabytes each. A line that names no origin is not a typed prompt, and a reply matters only
		// while a turn is open, so nothing else is decoded.
		if (bytes.Contains(raw, originKey) || open && bytes.Contains(raw, assistantWord)) && json.Unmarshal(raw, &l) == nil && !l.IsSidechain {
			switch said, typed := typedBy(l); {
			case typed:
				if open = said != "" && !skipped(l.Cwd, skip); open {
					turns = append(turns, turn{l.Timestamp, protocol.ReaderTurn{Tool: protocol.HarnessClaudeCode, Project: Cut(project(l.Cwd), projectMax), Said: Cut(said, saidMax)}})
				}
			case open && l.Type == "assistant":
				if text := replyText(l.Message); text != "" {
					turns[len(turns)-1].Reply = Cut(redact.Credentials(text), replyMax)
				}
			}
		}
		if err != nil {
			return turns
		}
	}
}

// typedBy reports whether the log marks the line as typed by a person, and what they typed once the harness's blocks are cut
// out and credentials replaced. said is empty for a slash command and for a prompt that was only harness blocks: such a line
// still ends the turn before it. A prompt typed with a pasted image is a list: what was typed is its text parts, joined. A list
// that holds a tool result is never typed, whatever its origin says.
func typedBy(l line) (said string, typed bool) {
	if l.Type != "user" || l.IsMeta || l.IsCompactSummary || l.Origin == nil || l.Origin.Kind != "human" {
		return "", false
	}
	var m struct {
		Content json.RawMessage `json:"content"`
	}
	var parts []part
	if json.Unmarshal(l.Message, &m) != nil || json.Unmarshal(m.Content, &said) != nil && json.Unmarshal(m.Content, &parts) != nil {
		return "", false
	}
	for _, p := range parts {
		if p.Type == "tool_result" {
			return "", false
		}
		if p.Type == "text" {
			said += p.Text
		}
	}
	said = strings.TrimSpace(harnessBlocks.ReplaceAllString(said, ""))
	for _, wrapper := range []string{"<command-name>", "<command-message>", "<local-command"} {
		if strings.HasPrefix(said, wrapper) {
			return "", true
		}
	}
	return strings.TrimSpace(redact.Credentials(said)), true
}

// replyText is the text parts of an assistant line, without its thinking and its tool calls.
func replyText(message json.RawMessage) string {
	var m struct {
		Content []part `json:"content"`
	}
	if json.Unmarshal(message, &m) != nil {
		return ""
	}
	var texts []string
	for _, p := range m.Content {
		if p.Type == "text" && strings.TrimSpace(p.Text) != "" {
			texts = append(texts, strings.TrimSpace(p.Text))
		}
	}
	return strings.Join(texts, " ")
}

// project names the folder a turn ran in: the repo's folder inside a harness worktree of it, the folder's last segment otherwise.
func project(cwd string) string {
	if m := worktree.FindStringSubmatch(cwd); m != nil {
		return m[1]
	}
	if cwd = strings.TrimRight(cwd, "/"); cwd == "" {
		return ""
	}
	return path.Base(cwd)
}

func skipped(cwd string, skip []string) bool {
	for _, name := range skip {
		if name != "" && strings.Contains(cwd, name) {
			return true
		}
	}
	return false
}

// Cut shortens s to max characters; a text that was cut ends in an ellipsis.
func Cut(s string, max int) string {
	if r := []rune(s); len(r) > max {
		return string(r[:max-1]) + "…"
	}
	return s
}
