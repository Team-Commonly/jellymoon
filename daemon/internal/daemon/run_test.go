package daemon

import (
	"bytes"
	"context"
	"encoding/json"
	"io"
	"log"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"syscall"
	"testing"
	"time"

	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

// A claim whose MCP URL is plain http to a host that is not local never reaches the agent: its program is not started, the
// run token appears in no log line, and the run is ended failed.
func TestRefusedMCPURLNeverStartsTheAgent(t *testing.T) {
	const runToken = "jmr_" + "0123456789012345678901234567890123456789012"
	var ended []string
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/v1/runs":
			_ = json.NewEncoder(w).Encode(protocol.ClaimRunResponse{RunID: "run_x", RunToken: runToken, Brief: "go", MCPURL: "http://example.invalid/mcp", ExpiresAt: "2040-01-01T00:00:00Z"})
		case "/v1/runs/run_x/end":
			var b protocol.EndRunRequest
			_ = json.NewDecoder(r.Body).Decode(&b)
			ended = append(ended, string(b.Outcome))
			_, _ = w.Write([]byte("{}"))
		default:
			http.NotFound(w, r)
		}
	}))
	defer srv.Close()
	marker := filepath.Join(t.TempDir(), "started")
	t.Setenv("JELLYMOON_HARNESS_FAKE", "/usr/bin/touch "+marker)
	c, err := api.New(srv.URL)
	if err != nil {
		t.Fatal(err)
	}
	var logs bytes.Buffer
	d := New(c, config.Store{Dir: t.TempDir()}, config.Computer{Token: "jmc_x"}, log.New(&logs, "", 0))
	d.ctx = context.Background()
	d.claim(config.Agent{AgentID: "agt_x", Harness: protocol.HarnessFake}, protocol.RunWake{WakeID: "wak_x"})
	if _, err := os.Stat(marker); err == nil {
		t.Errorf("the agent program was started for a refused MCP URL")
	}
	if strings.Join(ended, ",") != "failed" {
		t.Errorf("run ended %v, want one failed", ended)
	}
	if strings.Contains(logs.String(), runToken) || !strings.Contains(logs.String(), "refused") {
		t.Errorf("log must name the refusal and never the run token:\n%s", logs.String())
	}
}

func TestHarnessCommandNeedsAProgram(t *testing.T) {
	t.Setenv("JELLYMOON_HARNESS_CLAUDE_CODE", "")
	if cmd, err := harnessCommand(protocol.HarnessClaudeCode); err == nil || !strings.Contains(err.Error(), "JELLYMOON_HARNESS_CLAUDE_CODE") {
		t.Errorf("harnessCommand = %v, %v; want an error naming the variable to set", cmd, err)
	}
}

// The agent's environment is a short list: the daemon's own settings and any token variable stay behind.
func TestAgentEnvKeepsNoSecrets(t *testing.T) {
	t.Setenv("JELLYMOON_USER_TOKEN", "jmu_secret")
	t.Setenv("JELLYMOON_HARNESS_FAKE", "x")
	t.Setenv("PATH", "/usr/bin")
	got := strings.Join(agentEnv(), "\n")
	if strings.Contains(got, "jmu_") || strings.Contains(got, "JELLYMOON") || !strings.Contains(got, "PATH=/usr/bin") {
		t.Errorf("agentEnv = %q", got)
	}
}

// With none of the allowed names set, the agent still gets a list of its own: a nil one would mean the daemon's whole environment.
func TestAgentEnvIsEmptyNotNilWhenNothingIsAllowed(t *testing.T) {
	for _, k := range []string{"PATH", "HOME", "TMPDIR", "USER", "LANG", "LC_ALL", "TERM"} {
		if v, ok := os.LookupEnv(k); ok {
			os.Unsetenv(k)
			t.Cleanup(func() { os.Setenv(k, v) })
		}
	}
	t.Setenv("JELLYMOON_USER_TOKEN", "jmu_secret")
	t.Setenv("JELLYMOON_HOME", "/somewhere")
	got := agentEnv()
	if got == nil || len(got) != 0 {
		t.Errorf("agentEnv = %#v, want an empty, non-nil list", got)
	}
}

// A claim that is already past its expiresAt ends the run failed without starting the agent.
func TestExpiredClaimNeverStartsTheAgent(t *testing.T) {
	var ended []string
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/v1/runs":
			exp := time.Now().Add(-time.Minute).UTC().Format(time.RFC3339Nano)
			_ = json.NewEncoder(w).Encode(protocol.ClaimRunResponse{RunID: "run_x", RunToken: "jmr_x", Brief: "go", MCPURL: "http://127.0.0.1:1/mcp", ExpiresAt: exp})
		case "/v1/runs/run_x/end":
			var b protocol.EndRunRequest
			_ = json.NewDecoder(r.Body).Decode(&b)
			ended = append(ended, string(b.Outcome))
			_, _ = w.Write([]byte("{}"))
		default:
			http.NotFound(w, r)
		}
	}))
	defer srv.Close()
	marker := filepath.Join(t.TempDir(), "started")
	t.Setenv("JELLYMOON_HARNESS_FAKE", "/usr/bin/touch "+marker)
	c, err := api.New(srv.URL)
	if err != nil {
		t.Fatal(err)
	}
	var logs bytes.Buffer
	d := New(c, config.Store{Dir: t.TempDir()}, config.Computer{Token: "jmc_x"}, log.New(&logs, "", 0))
	d.ctx = context.Background()
	d.claim(config.Agent{AgentID: "agt_x", Harness: protocol.HarnessFake}, protocol.RunWake{WakeID: "wak_x"})
	// The log line is the deterministic signal: a started program may be killed before it touches the marker.
	if _, err := os.Stat(marker); err == nil || strings.Contains(logs.String(), "starting agent") {
		t.Errorf("the agent was started for a claim that had already expired:\n%s", logs.String())
	}
	if strings.Join(ended, ",") != "failed" {
		t.Errorf("run ended %v, want one failed", ended)
	}
}

// pidIsGone reports whether no process has the pid. Signal 0 only checks that the process exists.
func pidIsGone(pid int) bool { return syscall.Kill(pid, 0) == syscall.ESRCH }

// An agent that never answers is stopped when its run token expires: the run is ended failed, and the agent's process is gone.
// The expiry here is a second away; without the deadline the claim would wait on the agent for as long as the daemon runs.
func TestRunEndsWhenItsTokenExpires(t *testing.T) {
	var ended []string
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/v1/runs":
			exp := time.Now().Add(time.Second).UTC().Format(time.RFC3339Nano)
			_ = json.NewEncoder(w).Encode(protocol.ClaimRunResponse{RunID: "run_x", RunToken: "jmr_x", Brief: "go", MCPURL: "http://127.0.0.1:1/mcp", ExpiresAt: exp})
		case "/v1/runs/run_x/end":
			var b protocol.EndRunRequest
			_ = json.NewDecoder(r.Body).Decode(&b)
			ended = append(ended, string(b.Outcome))
			_, _ = w.Write([]byte("{}"))
		default:
			http.NotFound(w, r)
		}
	}))
	defer srv.Close()
	dir := t.TempDir()
	pidFile := filepath.Join(dir, "pid")
	script := filepath.Join(dir, "hung-agent.sh")
	// The agent records its pid and then ignores its turn: it never reads stdin and never answers.
	if err := os.WriteFile(script, []byte("#!/bin/sh\necho $$ > "+pidFile+"\nexec sleep 300\n"), 0o700); err != nil {
		t.Fatal(err)
	}
	t.Setenv("JELLYMOON_HARNESS_FAKE", script)
	c, err := api.New(srv.URL)
	if err != nil {
		t.Fatal(err)
	}
	d := New(c, config.Store{Dir: dir}, config.Computer{Token: "jmc_x"}, log.New(io.Discard, "", 0))
	ctx, cancel := context.WithCancel(context.Background())
	d.ctx = ctx
	done := make(chan struct{})
	go func() {
		d.claim(config.Agent{AgentID: "agt_x", Harness: protocol.HarnessFake}, protocol.RunWake{WakeID: "wak_x"})
		close(done)
	}()
	select {
	case <-done:
	case <-time.After(8 * time.Second):
		cancel() // stop the hung agent so the test can report
		<-done
		t.Fatal("the run outlived its token: the agent was still running 8 s after expiresAt")
	}
	cancel()
	if strings.Join(ended, ",") != "failed" {
		t.Errorf("run ended %v, want one failed", ended)
	}
	raw, err := os.ReadFile(pidFile)
	if err != nil {
		t.Fatalf("the agent never started: %v", err)
	}
	pid, _ := strconv.Atoi(strings.TrimSpace(string(raw)))
	if pid == 0 || !pidIsGone(pid) {
		t.Errorf("agent process %q is still running", raw)
	}
}
