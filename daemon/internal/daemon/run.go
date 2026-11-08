package daemon

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"

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

// onFrame handles one frame from a pod socket. Only run.wake matters here: it is acked, then claimed.
func (d *Daemon) onFrame(conn *ws.Conn, raw []byte) {
	var env protocol.Envelope
	if json.Unmarshal(raw, &env) != nil || env.Type != protocol.EventTypeRunWake {
		return
	}
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
	outcome := protocol.RunOutcomeCompleted
	if err := d.execute(a, c); err != nil {
		d.log.Printf("run %s: %v", c.RunID, strings.ReplaceAll(err.Error(), c.RunToken, "[run token]"))
		outcome = protocol.RunOutcomeFailed
	}
	if err := d.api.EndRun(d.comp.Token, c.RunID, outcome); err != nil {
		d.log.Printf("run %s: not ended: %v", c.RunID, err)
		return
	}
	d.log.Printf("run %s ended %s", c.RunID, outcome)
}

// execute runs the agent for a claimed run. The MCP URL is checked before the run token goes anywhere near the agent.
func (d *Daemon) execute(a config.Agent, c protocol.ClaimRunResponse) error {
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
	d.log.Printf("run %s: starting agent %s", c.RunID, a.AgentID)
	return acp.Run(d.ctx, acp.Spec{
		Command: cmd, Dir: dir, Env: agentEnv(), Prompt: c.Brief,
		Servers: []acp.MCPServer{{Type: "http", Name: "jellymoon", URL: mcpURL, Headers: []acp.Header{{Name: "Authorization", Value: "Bearer " + c.RunToken}}}},
	})
}
