package reading

import (
	"bytes"
	"context"
	_ "embed"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"regexp"
	"strings"
	"time"

	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
	"github.com/Team-Commonly/jellymoon/daemon/internal/reader"
)

// prompt is the whole system prompt of the reading call: a fixed text, so nothing in a day can change it.
//
//go:embed prompt.md
var prompt string

// modelShape is the only part of the command line a caller chooses: a name, which cannot start a flag or hold a space.
var modelShape = regexp.MustCompile(`^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$`)

// Encode writes v as one line of JSON with <, > and & left as they are: how `jellymoon day` prints and what the model reads.
func Encode(w io.Writer, v any) error {
	enc := json.NewEncoder(w)
	enc.SetEscapeHTML(false)
	return enc.Encode(v)
}

// call makes the reading call and returns the model's text. The harness is the person's own Claude Code, found on PATH, run
// with every tool, MCP server, slash command, saved session and user-level setting off: it can return text and do nothing else.
// The day goes in on stdin and nowhere else. The arguments are fixed but for the model's name, the environment is a short list
// of the daemon's (the harness's own sign-in is under HOME or CLAUDE_CONFIG_DIR), and the folder is new, empty and removed after.
func call(day protocol.ReaderDay, o Options) (string, error) {
	bin, err := exec.LookPath("claude")
	if err != nil {
		return "", fmt.Errorf("finding Claude Code: %w", err)
	}
	if err := os.MkdirAll(o.Dir, 0o700); err != nil {
		return "", err
	}
	dir, err := os.MkdirTemp(o.Dir, "call-")
	if err != nil {
		return "", err
	}
	defer os.RemoveAll(dir)
	var in, out, errOut bytes.Buffer
	if err := Encode(&in, day); err != nil {
		return "", err
	}
	ctx, cancel := context.WithTimeout(context.Background(), o.Timeout)
	defer cancel()
	cmd := exec.CommandContext(ctx, bin, "-p", "--model", o.Model, "--tools", "", "--strict-mcp-config", "--disable-slash-commands",
		"--no-session-persistence", "--setting-sources", "local", "--output-format", "json", "--system-prompt", prompt)
	cmd.Dir, cmd.Env, cmd.Stdin, cmd.Stdout, cmd.Stderr, cmd.WaitDelay = dir, []string{}, &in, &out, &errOut, time.Second
	for _, k := range []string{"PATH", "HOME", "TMPDIR", "USER", "LANG", "LC_ALL", "TERM", "CLAUDE_CONFIG_DIR"} {
		if v, ok := os.LookupEnv(k); ok {
			cmd.Env = append(cmd.Env, k+"="+v)
		}
	}
	err = cmd.Run()
	if ctx.Err() != nil {
		return "", fmt.Errorf("the reading call took longer than %s", o.Timeout)
	}
	if err != nil {
		return "", fmt.Errorf("the reading call failed: %w: %s", err, reader.Cut(strings.TrimSpace(errOut.String()), 300))
	}
	// --output-format json: one object, the model's text in result.
	var reply struct {
		IsError *bool  `json:"is_error"`
		Result  string `json:"result"`
	}
	if json.Unmarshal(out.Bytes(), &reply) != nil || reply.IsError == nil {
		return "", errors.New("the reading call printed something other than its result")
	}
	if *reply.IsError {
		return "", fmt.Errorf("the harness reported an error: %s", reader.Cut(reply.Result, 300))
	}
	return reply.Result, nil
}
