package daemon

import (
	"bytes"
	"context"
	"encoding/json"
	"log"
	"net/http"
	"net/http/httptest"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"

	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

// A shell agent that speaks just enough ACP to finish a turn. It writes its working directory (and branch, when it is in a
// repo) to $1 and what session/new carried to $1.new. With "fail" as $2 it leaves before answering, which fails the run.
const shellAgent = `pwd -P > "$1"
git rev-parse --abbrev-ref HEAD >> "$1" 2>/dev/null
if [ "$2" = fail ]; then exit 1; fi
read -r line
echo '{"jsonrpc":"2.0","id":1,"result":{}}'
read -r line
echo "$line" > "$1.new"
echo '{"jsonrpc":"2.0","id":2,"result":{"sessionId":"s"}}'
read -r line
echo '{"jsonrpc":"2.0","id":3,"result":{}}'
`

type wsRun struct {
	d      *Daemon
	store  config.Store
	record string
	logs   *bytes.Buffer
	ended  []string
}

// newWsRun is a daemon whose server hands out one claim, run_<id>, and whose fake harness is shellAgent.
func newWsRun(t *testing.T, runID, mode string) *wsRun {
	t.Helper()
	w := &wsRun{logs: &bytes.Buffer{}}
	srv := httptest.NewServer(http.HandlerFunc(func(rw http.ResponseWriter, r *http.Request) {
		switch {
		case r.URL.Path == "/v1/runs":
			_ = json.NewEncoder(rw).Encode(protocol.ClaimRunResponse{RunID: runID, RunToken: "jmr_" + strings.Repeat("a", 43), Brief: "go", MCPURL: "http://127.0.0.1:1/mcp", ExpiresAt: "2040-01-01T00:00:00Z"})
		case strings.HasSuffix(r.URL.Path, "/end"):
			var b protocol.EndRunRequest
			_ = json.NewDecoder(r.Body).Decode(&b)
			w.ended = append(w.ended, string(b.Outcome))
			_, _ = rw.Write([]byte("{}"))
		default:
			http.NotFound(rw, r)
		}
	}))
	t.Cleanup(srv.Close)
	c, err := api.New(srv.URL)
	if err != nil {
		t.Fatal(err)
	}
	script := filepath.Join(t.TempDir(), "agent.sh")
	if err := os.WriteFile(script, []byte(shellAgent), 0o600); err != nil {
		t.Fatal(err)
	}
	w.record = filepath.Join(t.TempDir(), "record")
	t.Setenv("JELLYMOON_HARNESS_FAKE", strings.Join([]string{"/bin/sh", script, w.record, mode}, " "))
	w.store = config.Store{Dir: filepath.Join(t.TempDir(), "cfg")}
	w.d = New(c, w.store, config.Computer{Token: "jmc_x"}, log.New(w.logs, "", 0))
	w.d.ctx = context.Background()
	return w
}

func (w *wsRun) claim(repo string) {
	w.d.claim(config.Agent{AgentID: "agt_x", Harness: protocol.HarnessFake, Repo: repo}, protocol.RunWake{WakeID: "wak_x"})
}

func (w *wsRun) root(t *testing.T) string {
	t.Helper()
	r, err := filepath.EvalSymlinks(w.store.WorkspaceRoot())
	if err != nil {
		t.Fatalf("the workspace root was never made: %v\n%s", err, w.logs)
	}
	return r
}

func (w *wsRun) recorded(t *testing.T) []string {
	t.Helper()
	raw, err := os.ReadFile(w.record)
	if err != nil {
		t.Fatalf("the agent never ran: %v\n%s", err, w.logs)
	}
	return strings.Fields(string(raw))
}

// The agent's directory is new, under the workspace root and named for the run, not the daemon's own; session/new says so too;
// and once the run completed the directory is gone.
func TestAgentRunsInAFreshDirUnderTheWorkspaceRoot(t *testing.T) {
	w := newWsRun(t, "run_one", "ok")
	w.claim("")
	got := w.recorded(t)
	want := filepath.Join(w.root(t), "run_one")
	here, _ := os.Getwd()
	if got[0] != want || got[0] == here {
		t.Errorf("the agent ran in %s, want %s (the daemon is in %s)", got[0], want, here)
	}
	if raw, _ := os.ReadFile(w.record + ".new"); !strings.Contains(string(raw), `"cwd":"`+want+`"`) {
		t.Errorf("session/new did not carry cwd %s: %s", want, raw)
	}
	if _, err := os.Lstat(want); err == nil {
		t.Errorf("the run directory is still there after a completed run")
	}
	if strings.Join(w.ended, ",") != "completed" {
		t.Errorf("run ended %v, want completed", w.ended)
	}
}

// With a repo configured, the agent's directory is a worktree of it on a branch for the run.
func TestAgentRunsInAWorktreeOfItsRepo(t *testing.T) {
	repo := t.TempDir()
	for _, args := range [][]string{{"init", "-q", "-b", "main"}, {"-c", "user.name=t", "-c", "user.email=t@example.invalid", "commit", "-q", "--allow-empty", "-m", "first"}} {
		cmd := exec.Command("git", append([]string{"-C", repo}, args...)...)
		if out, err := cmd.CombinedOutput(); err != nil {
			t.Fatalf("git %v: %v\n%s", args, err, out)
		}
	}
	w := newWsRun(t, "run_two", "ok")
	w.claim(repo)
	got := w.recorded(t)
	if len(got) != 2 || got[0] != filepath.Join(w.root(t), "run_two") || got[1] != "jellymoon/run-run_two" {
		t.Errorf("the agent saw %v, want a worktree named run_two on branch jellymoon/run-run_two", got)
	}
	if out, _ := exec.Command("git", "-C", repo, "branch", "--list", "jellymoon/*").Output(); len(out) != 0 {
		t.Errorf("the completed run left its branch: %s", out)
	}
}

// A failed run's directory is kept for inspection.
func TestFailedRunKeepsItsDir(t *testing.T) {
	w := newWsRun(t, "run_bad", "fail")
	w.claim("")
	if _, err := os.Lstat(filepath.Join(w.root(t), "run_bad")); err != nil {
		t.Errorf("the failed run's directory is gone: %v", err)
	}
	if strings.Join(w.ended, ",") != "failed" {
		t.Errorf("run ended %v, want failed", w.ended)
	}
}

// A directory left by a daemon that was killed mid-run is swept when the next daemon starts.
func TestStartSweepsStaleRunDirs(t *testing.T) {
	w := newWsRun(t, "run_x", "ok")
	stale := filepath.Join(w.store.WorkspaceRoot(), "run_dead")
	if err := os.MkdirAll(stale, 0o700); err != nil {
		t.Fatal(err)
	}
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	if err := os.MkdirAll(w.store.Dir, 0o700); err != nil {
		t.Fatal(err)
	}
	if err := w.d.Run(ctx); err != nil {
		t.Fatal(err)
	}
	if _, err := os.Lstat(stale); err == nil {
		t.Errorf("the stale run directory survived the daemon's start")
	}
}
