package daemon

import (
	"bytes"
	"context"
	"encoding/json"
	"log"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"

	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

const (
	recordingPath = "../../testdata/acp/claude-slice.jsonl"
	goldenPath    = "../../testdata/acp/claude-slice.golden"
	computerToken = "jmc_" + "0123456789012345678901234567890123456789012"
	runToken      = "jmr_" + "9876543210987654321098765432109876543210987"
)

// replayPod is the pod, as far as one run can see it: it hands out a run, answers the agent's MCP calls with what the real pod
// answered in the recording, and writes down every call it receives, in order. Nothing is judged here; the test compares the list.
type replayPod struct {
	*httptest.Server
	mu      sync.Mutex
	calls   []string
	answers []entry // the recorded MCP exchanges, in order
	served  int     // how many of them the agent has asked for
	brief   string
}

func newReplayPod(t *testing.T, ents []entry) *replayPod {
	p := &replayPod{}
	for _, e := range ents {
		switch {
		case e.T == "mcp":
			p.answers = append(p.answers, e)
		case e.T == "acp" && e.Dir == "in":
			var m rpc
			_ = json.Unmarshal(e.Msg, &m)
			var pr struct{ Prompt []struct{ Text string } }
			if m.Method == "session/prompt" && json.Unmarshal(m.Params, &pr) == nil && len(pr.Prompt) == 1 {
				p.brief = pr.Prompt[0].Text
			}
		}
	}
	mux := http.NewServeMux()
	mux.HandleFunc("POST /v1/runs", p.claim)
	mux.HandleFunc("POST /v1/runs/{run}/end", p.end)
	mux.HandleFunc("/mcp", p.mcp)
	p.Server = httptest.NewServer(mux)
	t.Cleanup(p.Close)
	return p
}

func (p *replayPod) note(line string) {
	p.mu.Lock()
	defer p.mu.Unlock()
	p.calls = append(p.calls, line)
}

// authorized says whether the request carries the token this kind of caller must: the computer token on the daemon's own calls, the
// run token on the agent's. A call that does not is written down as refused, which makes the list differ from the golden one.
func (p *replayPod) authorized(w http.ResponseWriter, r *http.Request, token string) bool {
	if r.Header.Get("Authorization") == "Bearer "+token {
		return true
	}
	p.note("REFUSED " + r.Method + " " + r.URL.Path + ": wrong Authorization")
	http.Error(w, `{"code":"unauthorized","message":"wrong token"}`, http.StatusUnauthorized)
	return false
}

func (p *replayPod) claim(w http.ResponseWriter, r *http.Request) {
	if !p.authorized(w, r, computerToken) {
		return
	}
	var req protocol.ClaimRunRequest
	_ = json.NewDecoder(r.Body).Decode(&req)
	p.note("POST /v1/runs wakeId=" + req.WakeID)
	_ = json.NewEncoder(w).Encode(protocol.ClaimRunResponse{RunID: "run_00000000000000000000000004", RunToken: runToken, Brief: p.brief,
		MCPURL: p.URL + "/mcp", ExpiresAt: "2040-01-01T00:00:00Z"})
}

func (p *replayPod) end(w http.ResponseWriter, r *http.Request) {
	if !p.authorized(w, r, computerToken) {
		return
	}
	var req protocol.EndRunRequest
	_ = json.NewDecoder(r.Body).Decode(&req)
	p.note("POST /v1/runs/" + r.PathValue("run") + "/end outcome=" + string(req.Outcome))
	_, _ = w.Write([]byte("{}"))
}

// mcp writes down the call (the JSON-RPC method, and for a tool call its name and arguments) and answers with the recorded exchange
// at the same position.
func (p *replayPod) mcp(w http.ResponseWriter, r *http.Request) {
	if !p.authorized(w, r, runToken) {
		return
	}
	var req struct {
		Method string
		Params struct {
			Name      string
			Arguments json.RawMessage
		}
	}
	var body bytes.Buffer
	_, _ = body.ReadFrom(r.Body)
	_ = json.Unmarshal(body.Bytes(), &req)
	line := r.Method + " /mcp"
	if req.Method != "" {
		line += " " + req.Method
	}
	if req.Params.Name != "" {
		line += " " + req.Params.Name + " " + compact(req.Params.Arguments)
	}
	p.mu.Lock()
	p.calls = append(p.calls, line)
	p.served++
	n := p.served
	p.mu.Unlock()
	if n > len(p.answers) {
		http.Error(w, `{"code":"unexpected","message":"more MCP calls than the recording has"}`, http.StatusInternalServerError)
		return
	}
	a := p.answers[n-1]
	var text string
	if json.Unmarshal(a.Res, &text) != nil {
		text = string(a.Res) // JSON the pod sent; a string here is a body that was not JSON (empty, for a notification)
	}
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(a.Status)
	_, _ = w.Write([]byte(text))
}

func compact(raw json.RawMessage) string {
	var v any
	if json.Unmarshal(raw, &v) != nil {
		return string(raw)
	}
	b, _ := json.Marshal(v) // keys sorted, so the line does not depend on the order the model wrote them
	return string(b)
}

// TestReplayClaude drives the daemon through one claimed run whose agent is a real Claude Code session replayed from its recording
// (testdata/acp/claude-slice.jsonl, made by scripts/record-acp.mjs against the pinned claude-agent-acp adapter). The daemon starts the
// agent, speaks ACP to it, hands it the run token in session/new, and ends the run; the pod's list of calls must be the golden one.
func TestReplayClaude(t *testing.T) {
	ents, err := loadRecording(recordingPath)
	if err != nil {
		t.Fatal(err)
	}
	pod := newReplayPod(t, ents)
	exe, err := os.Executable()
	if err != nil {
		t.Fatal(err)
	}
	report := filepath.Join(t.TempDir(), "report")
	abs, _ := filepath.Abs(recordingPath)
	t.Setenv("JELLYMOON_HARNESS_CLAUDE_CODE", strings.Join([]string{exe, "-test.run=^TestReplayAgent$", "-replay.agent=" + abs, "-replay.report=" + report}, " "))
	c, err := api.New(pod.URL)
	if err != nil {
		t.Fatal(err)
	}
	var logs bytes.Buffer
	d := New(c, config.Store{Dir: t.TempDir()}, config.Computer{Token: computerToken}, log.New(&logs, "", 0))
	d.ctx = context.Background()
	d.claim(config.Agent{AgentID: "agt_00000000000000000000000003", Harness: protocol.HarnessClaudeCode}, protocol.RunWake{WakeID: "wak_replay"})

	if why, err := os.ReadFile(report); err == nil {
		t.Errorf("the replaying agent stopped: %s", why)
	}
	golden, err := os.ReadFile(goldenPath)
	if err != nil {
		t.Fatal(err)
	}
	if got, want := strings.Join(pod.calls, "\n")+"\n", string(golden); got != want {
		t.Errorf("the pod's calls are not the golden list.\n--- got\n%s--- want (%s)\n%s", got, goldenPath, want)
	}
	if strings.Contains(logs.String(), runToken) || strings.Contains(logs.String(), computerToken) {
		t.Errorf("a token reached the daemon's log:\n%s", logs.String())
	}
}
