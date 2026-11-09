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
	"sync"
	"testing"
	"time"

	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

var fakeAgent string // the e2e suite's fake ACP agent, built once for these tests

func TestMain(m *testing.M) {
	dir, err := os.MkdirTemp("", "fakeagent")
	if err != nil {
		log.Fatal(err)
	}
	fakeAgent = filepath.Join(dir, "fakeagent")
	build := exec.Command("go", "build", "-o", fakeAgent, "./e2e/testdata/fakeagent")
	build.Dir = "../../.."
	if out, err := build.CombinedOutput(); err != nil {
		log.Fatalf("building the fake agent: %v\n%s", err, out)
	}
	code := m.Run()
	_ = os.RemoveAll(dir)
	os.Exit(code)
}

// world is a daemon against a stub pod that hands out one run and records how the run was ended.
type world struct {
	d      *Daemon
	cancel context.CancelFunc
	logs   *bytes.Buffer
	rec    string
	mu     sync.Mutex
	ended  []string
}

type agentRecord struct {
	Prompt     json.RawMessage
	Permission json.RawMessage
	Cancelled  bool
	Signalled  bool
}

func newWorld(t *testing.T, expires time.Time, agentArgs ...string) *world {
	w := &world{logs: &bytes.Buffer{}, rec: filepath.Join(t.TempDir(), "rec.json")}
	srv := httptest.NewServer(http.HandlerFunc(func(rw http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/v1/runs":
			_ = json.NewEncoder(rw).Encode(protocol.ClaimRunResponse{RunID: "run_x", RunToken: "jmr_" + strings.Repeat("a", 43), Brief: "go", MCPURL: "http://127.0.0.1:1/mcp", ExpiresAt: expires.UTC().Format(time.RFC3339Nano)})
		case "/v1/runs/run_x/end":
			var b protocol.EndRunRequest
			_ = json.NewDecoder(r.Body).Decode(&b)
			w.mu.Lock()
			w.ended = append(w.ended, string(b.Outcome))
			w.mu.Unlock()
			_, _ = rw.Write([]byte("{}"))
		default:
			http.NotFound(rw, r)
		}
	}))
	t.Cleanup(srv.Close)
	t.Setenv("JELLYMOON_HARNESS_FAKE", strings.Join(append([]string{fakeAgent, w.rec}, agentArgs...), " "))
	c, err := api.New(srv.URL)
	if err != nil {
		t.Fatal(err)
	}
	w.d = New(c, config.Store{Dir: t.TempDir()}, config.Computer{Token: "jmc_x"}, log.New(w.logs, "", 0))
	var ctx context.Context
	ctx, w.cancel = context.WithCancel(context.Background())
	t.Cleanup(w.cancel)
	w.d.ctx = ctx
	return w
}

// start runs the claim in the background and returns a channel that closes when the run is over.
func (w *world) start(allow bool) <-chan struct{} {
	done := make(chan struct{})
	go func() {
		defer close(done)
		w.d.claim(config.Agent{AgentID: "agt_x", PodID: "pod_x", Harness: protocol.HarnessFake, AllowPermissions: allow}, protocol.RunWake{WakeID: "wak_x"})
	}()
	return done
}

func (w *world) record() agentRecord {
	var r agentRecord
	raw, _ := os.ReadFile(w.rec)
	_ = json.Unmarshal(raw, &r)
	return r
}

// until waits for the fake agent's record to satisfy ok.
func (w *world) until(t *testing.T, what string, ok func(agentRecord) bool) agentRecord {
	t.Helper()
	for end := time.Now().Add(15 * time.Second); time.Now().Before(end); time.Sleep(10 * time.Millisecond) {
		if r := w.record(); ok(r) {
			return r
		}
	}
	t.Fatalf("the fake agent never reached %s: %+v\n%s", what, w.record(), w.logs)
	return agentRecord{}
}

func (w *world) finish(t *testing.T, done <-chan struct{}) {
	t.Helper()
	select {
	case <-done:
	case <-time.After(15 * time.Second):
		t.Fatalf("the run did not end\n%s", w.logs)
	}
}

func (w *world) outcome() string {
	w.mu.Lock()
	defer w.mu.Unlock()
	return strings.Join(w.ended, ",")
}

func asking(r agentRecord) bool { return len(r.Prompt) > 0 && string(r.Prompt) != "null" }

// run.stop for a running run cancels the session and the run ends cancelled (ADR 0002).
func TestRunStopCancelsTheSessionAndEndsTheRunCancelled(t *testing.T) {
	w := newWorld(t, time.Now().Add(time.Hour), "cancel")
	done := w.start(false)
	w.until(t, "its prompt", asking)
	data, _ := json.Marshal(protocol.RunStop{RunID: "run_x"})
	raw, _ := json.Marshal(protocol.Envelope{V: 1, Type: protocol.EventTypeRunStop, PodID: "pod_x", Data: data})
	w.d.onFrame(nil, raw)
	w.finish(t, done)
	if r := w.record(); !r.Cancelled || r.Signalled {
		t.Errorf("agent cancelled=%v signalled=%v, want cancelled and not signalled", r.Cancelled, r.Signalled)
	}
	if got := w.outcome(); got != "cancelled" {
		t.Errorf("run ended %q, want cancelled", got)
	}
}

// A run stop for another run, or one that is not running, does nothing.
func TestRunStopForAnotherRunIsIgnored(t *testing.T) {
	w := newWorld(t, time.Now().Add(time.Hour), "cancel")
	done := w.start(false)
	w.until(t, "its prompt", asking)
	data, _ := json.Marshal(protocol.RunStop{RunID: "run_other"})
	raw, _ := json.Marshal(protocol.Envelope{V: 1, Type: protocol.EventTypeRunStop, PodID: "pod_x", Data: data})
	w.d.onFrame(nil, raw)
	time.Sleep(300 * time.Millisecond)
	if w.record().Cancelled {
		t.Errorf("a stop for another run cancelled this one")
	}
	w.cancel() // end the test's run through the daemon stop
	w.finish(t, done)
}

// The run's deadline (the run token's expiry) cancels the session first, and the run ends failed.
func TestDeadlineCancelsTheSessionAndEndsTheRunFailed(t *testing.T) {
	w := newWorld(t, time.Now().Add(2*time.Second), "cancel")
	done := w.start(false)
	w.finish(t, done)
	if r := w.record(); !r.Cancelled || r.Signalled {
		t.Errorf("agent cancelled=%v signalled=%v, want cancelled and not signalled", r.Cancelled, r.Signalled)
	}
	if got := w.outcome(); got != "failed" {
		t.Errorf("run ended %q, want failed", got)
	}
}

// A stopping daemon cancels the session before it signals, and the run ends failed (ADR 0005).
func TestDaemonStopCancelsTheSessionAndEndsTheRunFailed(t *testing.T) {
	w := newWorld(t, time.Now().Add(time.Hour), "cancel")
	done := w.start(false)
	w.until(t, "its prompt", asking)
	w.cancel()
	w.finish(t, done)
	if r := w.record(); !r.Cancelled || r.Signalled {
		t.Errorf("agent cancelled=%v signalled=%v, want cancelled and not signalled", r.Cancelled, r.Signalled)
	}
	if got := w.outcome(); got != "failed" {
		t.Errorf("run ended %q, want failed", got)
	}
}

// The allow setting reaches the agent's permission request: rejected by default, allow_once with the setting. The log has the
// request and the answer, and never the tool call's title.
func TestAllowSettingDecidesThePermissionAnswer(t *testing.T) {
	for allow, want := range map[bool]string{false: "reject_once", true: "allow_once"} {
		w := newWorld(t, time.Now().Add(time.Hour), "permission=allow_always,allow_once,reject_once")
		w.finish(t, w.start(allow))
		if got := string(w.record().Permission); !strings.Contains(got, `"optionId":"`+want+`"`) {
			t.Errorf("allow=%v: the agent was answered %s, want %s", allow, got, want)
		}
		if log := w.logs.String(); !strings.Contains(log, "permission request") || !strings.Contains(log, want) || strings.Contains(log, "a tool call title") {
			t.Errorf("allow=%v: log must name the request and the answer and not the title:\n%s", allow, log)
		}
	}
}

// An agent added with the allow setting keeps it across a reload.
func TestAddKeepsTheAllowSetting(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(rw http.ResponseWriter, r *http.Request) {
		_ = json.NewEncoder(rw).Encode(protocol.RegisterAgentResponse{AgentID: "agt_1"})
	}))
	defer srv.Close()
	c, _ := api.New(srv.URL)
	store := config.Store{Dir: t.TempDir()}
	d := New(c, store, config.Computer{Token: "jmc_x"}, log.New(&bytes.Buffer{}, "", 0))
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	d.ctx = ctx
	if _, err := d.add(AddRequest{PodID: "pod_a", Harness: protocol.HarnessFake, DisplayName: "a", AllowPermissions: true}); err != nil {
		t.Fatal(err)
	}
	d.wg.Wait()
	if got, _ := store.Agents(); len(got) != 1 || !got[0].AllowPermissions {
		t.Errorf("stored agents = %+v, want one with the allow setting", got)
	}
}
