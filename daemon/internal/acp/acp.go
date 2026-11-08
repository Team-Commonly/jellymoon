// Package acp runs one agent turn over the Agent Client Protocol: JSON-RPC 2.0, one message per line, on the agent's stdin and
// stdout. The daemon is the client. It starts the agent program in its own process group, sends initialize, session/new (the
// MCP servers the agent may use, which is where the run token travels) and session/prompt (the brief), waits for the turn to
// end, and always takes the whole process group down afterwards. Standard library only.
//
// The agent may ask the client for things while a turn runs. The only request answered is session/request_permission (policy in
// permission.go); every other request gets -32601. When the context ends mid-turn the client sends session/cancel first and gives
// the turn grace to end, and only then signals the process group (docs/adr/0007-agent-permission-and-cancel.md).
package acp

import (
	"bufio"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"strconv"
	"syscall"
	"time"
)

// grace is how long an agent gets to leave on its own: after its stdin closes at the end of a turn, and again after SIGTERM.
const grace = 2 * time.Second

// Header is one HTTP header an MCP server is called with.
type Header struct {
	Name  string `json:"name"`
	Value string `json:"value"`
}

// MCPServer is an MCP server over HTTP that the agent is told to use.
type MCPServer struct {
	Type    string   `json:"type"`
	Name    string   `json:"name"`
	URL     string   `json:"url"`
	Headers []Header `json:"headers"`
}

// Spec is one turn: the program to start (argv, never a secret), its working directory and environment, the MCP servers and the prompt.
type Spec struct {
	Command []string
	Dir     string
	Env     []string
	Servers []MCPServer
	Prompt  string
	// Allow answers the agent's permission requests with their allow_once option. The default is to reject them.
	Allow bool
	// Logf, when set, is told each permission request and its answer. It is never given a tool's arguments.
	Logf func(format string, args ...any)
}

type message struct {
	ID     json.RawMessage `json:"id"`
	Method string          `json:"method"`
	Params json.RawMessage `json:"params"`
	Result json.RawMessage `json:"result"`
	Error  *struct {
		Code    int    `json:"code"`
		Message string `json:"message"`
	} `json:"error"`
}

type conn struct {
	in         io.Writer
	msgs       <-chan message
	next       int
	allow      bool
	logf       func(string, ...any)
	sessionID  string // set once session/new answered
	cancelling bool   // session/cancel has been sent: the turn is being abandoned
}

func (c *conn) write(v map[string]any) error {
	v["jsonrpc"] = "2.0"
	b, err := json.Marshal(v)
	if err != nil {
		return err
	}
	_, err = c.in.Write(append(b, '\n'))
	return err
}

// call sends a request and waits for its answer. A request the agent makes meanwhile is answered by serve. When ctx ends while
// session/prompt is outstanding, the turn is cancelled (session/cancel) and given grace to end; before that, ctx ends the call at once.
func (c *conn) call(ctx context.Context, method string, params any) (json.RawMessage, error) {
	c.next++
	if err := c.write(map[string]any{"id": c.next, "method": method, "params": params}); err != nil {
		return nil, fmt.Errorf("%s: %w", method, err)
	}
	done := ctx.Done()
	var expired <-chan time.Time
	for {
		select {
		case <-done:
			if method != "session/prompt" || c.sessionID == "" {
				return nil, context.Cause(ctx)
			}
			c.cancelling, done = true, nil
			_ = c.write(map[string]any{"method": "session/cancel", "params": map[string]any{"sessionId": c.sessionID}})
			t := time.NewTimer(grace)
			defer t.Stop()
			expired = t.C
		case <-expired:
			return nil, context.Cause(ctx)
		case m, ok := <-c.msgs:
			switch {
			case !ok:
				return nil, fmt.Errorf("%s: the agent exited before it answered", method)
			case m.Method != "" && m.ID != nil:
				c.serve(m)
			case m.Method == "" && string(m.ID) == strconv.Itoa(c.next):
				if m.Error != nil {
					return nil, fmt.Errorf("%s: agent error %d: %s", method, m.Error.Code, m.Error.Message)
				}
				return m.Result, nil
			}
		}
	}
}

func lines(r io.Reader, done <-chan struct{}) <-chan message {
	out := make(chan message)
	go func() {
		defer close(out)
		sc := bufio.NewScanner(r)
		sc.Buffer(make([]byte, 1<<20), 1<<20)
		for sc.Scan() {
			var m message
			if json.Unmarshal(sc.Bytes(), &m) != nil {
				continue
			}
			select {
			case out <- m:
			case <-done:
				return
			}
		}
	}()
	return out
}

// command builds the agent's process: its own process group, and exactly the environment the Spec names. A nil Env would give the
// agent the daemon's whole environment, so it becomes an empty one.
func command(s Spec) *exec.Cmd {
	cmd := exec.Command(s.Command[0], s.Command[1:]...)
	cmd.Dir, cmd.Env = s.Dir, s.Env
	if cmd.Env == nil {
		cmd.Env = []string{}
	}
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	return cmd
}

// Run starts the agent, runs one turn and returns nil when the turn ended. When ctx ends first, or the agent fails, it returns
// an error. Either way the agent's whole process group is gone before it returns.
func Run(ctx context.Context, s Spec) error {
	if len(s.Command) == 0 {
		return errors.New("no agent command")
	}
	cmd := command(s)
	in, err := cmd.StdinPipe()
	if err != nil {
		return err
	}
	pr, pw, err := os.Pipe()
	if err != nil {
		return err
	}
	cmd.Stdout = pw // the read end is ours, so Wait never closes it under the reader
	if err := cmd.Start(); err != nil {
		pr.Close()
		pw.Close()
		return fmt.Errorf("starting the agent: %w", err)
	}
	pw.Close()
	exited := make(chan struct{})
	go func() { _ = cmd.Wait(); close(exited) }()
	stop := make(chan struct{})
	defer close(stop)
	c := &conn{in: in, msgs: lines(pr, stop), allow: s.Allow, logf: s.Logf}
	err = turn(ctx, c, s)
	_ = in.Close()
	if err == nil {
		wait(exited, grace) // a finished agent leaves when its stdin closes
		if c.cancelling {
			err = context.Cause(ctx) // the turn ended on cancel: the run was still cut short
		}
	}
	_ = syscall.Kill(-cmd.Process.Pid, syscall.SIGTERM)
	if !wait(exited, grace) {
		_ = syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL)
		<-exited
	}
	_ = syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL) // descendants that outlived the leader
	pr.Close()
	return err
}

func wait(ch <-chan struct{}, d time.Duration) bool {
	select {
	case <-ch:
		return true
	case <-time.After(d):
		return false
	}
}

func turn(ctx context.Context, c *conn, s Spec) error {
	if _, err := c.call(ctx, "initialize", map[string]any{"protocolVersion": 1, "clientCapabilities": map[string]any{}}); err != nil {
		return err
	}
	res, err := c.call(ctx, "session/new", map[string]any{"cwd": s.Dir, "mcpServers": s.Servers})
	if err != nil {
		return err
	}
	var ses struct {
		SessionID string `json:"sessionId"`
	}
	if err := json.Unmarshal(res, &ses); err != nil || ses.SessionID == "" {
		return errors.New("session/new: the agent returned no sessionId")
	}
	c.sessionID = ses.SessionID
	_, err = c.call(ctx, "session/prompt", map[string]any{"sessionId": ses.SessionID, "prompt": []map[string]any{{"type": "text", "text": s.Prompt}}})
	return err
}
