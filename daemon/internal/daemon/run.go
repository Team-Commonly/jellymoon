package daemon

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"strings"
	"time"

	"github.com/Team-Commonly/jellymoon/daemon/internal/acp"
	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
	"github.com/Team-Commonly/jellymoon/daemon/internal/ws"
)

// A harness runs as a program that speaks ACP on stdio. JELLYMOON_HARNESS_<NAME> (the harness name in capitals, - as _) holds
// that program and its arguments, split on spaces. It is for the fake harness today; the real harnesses get their own defaults.
// The command line is argv, so it never holds a token.
const harnessEnvPrefix = "JELLYMOON_HARNESS_"

func harnessCommand(h protocol.Harness) ([]string, error) {
	name := harnessEnvPrefix + strings.ToUpper(strings.ReplaceAll(string(h), "-", "_"))
	if cmd := strings.Fields(os.Getenv(name)); len(cmd) > 0 {
		return cmd, nil
	}
	return nil, fmt.Errorf("no program is configured for harness %s; set %s", h, name)
}

// agentEnv is the whole environment an agent gets: a short list of the daemon's, never the rest. The daemon's environment can
// hold the user token or settings of its own, and the agent's only credential is the run token in its MCP server config.
// The result is never nil: os/exec gives a child whose Env is nil the whole parent environment, so "none of them set" must be
// an empty list, not an absent one.
func agentEnv() []string {
	env := []string{}
	for _, k := range []string{"PATH", "HOME", "TMPDIR", "USER", "LANG", "LC_ALL", "TERM"} {
		if v, ok := os.LookupEnv(k); ok {
			env = append(env, k+"="+v)
		}
	}
	return env
}

// Why a run's context ended, as the cause the ACP client hands back. Only a stop makes the run `cancelled`.
var (
	errStopped  = errors.New("the run was stopped")
	errDeadline = errors.New("the run reached its deadline")
)

// onFrame handles one frame from a pod socket. run.wake is acked, then claimed; run.stop cancels that run's session.
func (d *Daemon) onFrame(conn *ws.Conn, raw []byte) {
	var env protocol.Envelope
	if json.Unmarshal(raw, &env) != nil {
		return
	}
	switch env.Type {
	case protocol.EventTypeRunWake:
		d.onWake(conn, env)
	case protocol.EventTypeRunStop:
		var s protocol.RunStop
		if json.Unmarshal(env.Data, &s) != nil {
			d.log.Printf("pod %s: unreadable run.stop", env.PodID)
			return
		}
		d.mu.Lock()
		cancel := d.runs[s.RunID]
		d.mu.Unlock()
		if cancel != nil { // a run that is not running here (ended, or claimed by nobody yet) has nothing to cancel
			d.log.Printf("run %s: stopped", s.RunID)
			cancel(errStopped)
		}
	}
}

// onWake acks a run.wake and starts its run.
func (d *Daemon) onWake(conn *ws.Conn, env protocol.Envelope) {
	var w protocol.RunWake
	if err := json.Unmarshal(env.Data, &w); err != nil {
		d.log.Printf("pod %s: unreadable run.wake: %v", env.PodID, err)
		return
	}
	ack, _ := json.Marshal(protocol.WakeAck{WakeID: w.WakeID})
	b, _ := json.Marshal(protocol.Envelope{V: 1, Type: protocol.EventTypeWakeAck, PodID: env.PodID, Data: ack})
	_ = conn.Send(b)
	d.begin(w)
}

// begin starts the run for a wake once. A wake that is re-sent while its run is being claimed is the same wake.
func (d *Daemon) begin(w protocol.RunWake) {
	agents, err := d.store.Agents()
	if err != nil {
		d.log.Printf("wake %s: %v", w.WakeID, err)
		return
	}
	var agent *config.Agent
	for i := range agents {
		if agents[i].AgentID == w.AgentID {
			agent = &agents[i]
		}
	}
	d.mu.Lock()
	defer d.mu.Unlock()
	if agent == nil || d.waking[w.WakeID] {
		return
	}
	d.waking[w.WakeID] = true
	d.wg.Add(1)
	go func() {
		defer d.wg.Done()
		d.claim(*agent, w)
		d.mu.Lock()
		delete(d.waking, w.WakeID)
		d.mu.Unlock()
	}()
}

// claim turns a wake into a run: POST /v1/runs, start the agent with the run token, and tell the pod how it ended.
func (d *Daemon) claim(a config.Agent, w protocol.RunWake) {
	c, err := d.api.ClaimRun(d.comp.Token, w.WakeID)
	if err != nil {
		d.log.Printf("wake %s: not claimed: %v", w.WakeID, err)
		return
	}
	ctx, done := d.runContext(c)
	outcome := protocol.RunOutcomeCompleted
	if err := d.execute(ctx, a, c); err != nil {
		d.log.Printf("run %s: %v", c.RunID, strings.ReplaceAll(err.Error(), c.RunToken, "[run token]"))
		outcome = protocol.RunOutcomeFailed
		if errors.Is(err, errStopped) {
			outcome = protocol.RunOutcomeCancelled // ADR 0002: a stopped run ends cancelled; a deadline or a daemon stop is a failure
		}
	}
	done()
	if err := d.api.EndRun(d.comp.Token, c.RunID, outcome); err != nil {
		d.log.Printf("run %s: not ended: %v", c.RunID, err)
		return
	}
	d.log.Printf("run %s ended %s", c.RunID, outcome)
}

// runContext is the context one run lives in. It ends when the daemon stops, when a person stops the run (run.stop finds it by
// run id). execute adds the deadline, the expiry of the run token. The cause says which. done releases it.
func (d *Daemon) runContext(c protocol.ClaimRunResponse) (context.Context, func()) {
	ctx, cancel := context.WithCancelCause(d.ctx)
	d.mu.Lock()
	d.runs[c.RunID] = cancel
	d.mu.Unlock()
	return ctx, func() {
		d.mu.Lock()
		delete(d.runs, c.RunID)
		d.mu.Unlock()
		cancel(nil)
	}
}

// execute runs the agent for a claimed run. The MCP URL is checked before the run token goes anywhere near the agent.
func (d *Daemon) execute(ctx context.Context, a config.Agent, c protocol.ClaimRunResponse) error {
	mcpURL, err := api.CheckMCP(c.MCPURL)
	if err != nil {
		return err
	}
	cmd, err := harnessCommand(a.Harness)
	if err != nil {
		return err
	}
	dir, err := os.Getwd()
	if err != nil {
		return err
	}
	// The run token stops working at expiresAt, so the agent stops with it: a hung agent cannot outlive its credential. A
	// timestamp that cannot be read means the token cannot be bounded, so the run is refused before the agent starts.
	expires, err := time.Parse(time.RFC3339Nano, c.ExpiresAt)
	if err != nil {
		return fmt.Errorf("the claim's expiresAt %q is not a timestamp: %w", c.ExpiresAt, err)
	}
	if !time.Now().Before(expires) {
		return fmt.Errorf("the claim expired at %s, before the agent could start", c.ExpiresAt)
	}
	ctx, cancel := context.WithDeadlineCause(ctx, expires, errDeadline) // inside the run's context, so a stop or a daemon stop still ends it first
	defer cancel()
	d.log.Printf("run %s: starting agent %s", c.RunID, a.AgentID)
	return acp.Run(ctx, acp.Spec{
		Command: cmd, Dir: dir, Env: agentEnv(), Prompt: c.Brief, Allow: a.AllowPermissions,
		Logf:    func(f string, args ...any) { d.log.Printf("run %s: "+f, append([]any{c.RunID}, args...)...) },
		Servers: []acp.MCPServer{{Type: "http", Name: "jellymoon", URL: mcpURL, Headers: []acp.Header{{Name: "Authorization", Value: "Bearer " + c.RunToken}}}},
	})
}
