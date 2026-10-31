package e2e

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
)

// f11 is F09's world with a scripted agent: the fake agent makes the MCP calls in `script`, in order, and records every reply.
// The agent is held up, so its run token stays live for the plain HTTP calls the test makes with it.
type f11 struct {
	*f09
	script []map[string]any
	record string
	rec    agentRec
}

// askAt is where the one valid ask sits in the script: the pod has seen that many MCP calls (refused ones too) before it.
const askAt = 6

var (
	longHeadline  = strings.Repeat("x", 121)
	exactHeadline = strings.Repeat("y", 120)
	decisionID    = regexp.MustCompile(`^dec_[0-9A-HJKMNP-TV-Z]{26}$`)
	stamp         = regexp.MustCompile(`^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(\.\d+)?Z$`)
)

func step11(tool string, args map[string]any) map[string]any {
	return map[string]any{"tool": tool, "arguments": args}
}

// plus copies m with k set to v (or removed when v is nil).
func plus(m map[string]any, k string, v any) map[string]any {
	out := map[string]any{}
	for key, val := range m {
		out[key] = val
	}
	if v == nil {
		delete(out, k)
	} else {
		out[k] = v
	}
	return out
}

// validAsk is the card of schema/fixtures/Message/valid-ask-1.json, as the poster writes it.
func validAsk(summary string) map[string]any {
	claim := map[string]any{"text": "The fix passed 212 of 212 tests.", "toolCalls": []string{"tc_01JCZK7Q8XG4M2B9V5T3W6RH01"}}
	return map[string]any{
		"summary": summary, "kind": "ask", "status": "blocked",
		"needs": map[string]any{"to": map[string]any{"role": "owner"}, "action": "approve"},
		"options": []any{
			map[string]any{"id": "a", "label": "Merge now", "consequence": "The agent merges on its next run."},
			map[string]any{"id": "b", "label": "Wait", "consequence": "The agent waits for review."},
		},
		"context": map[string]any{
			"happened":       []any{claim},
			"checked":        []any{},
			"stakes":         map[string]any{"text": "Waiting holds the release a day.", "messages": []string{"msg_01JCZK7Q8XG4M2B9V5T3W6RH01"}},
			"recommendation": map[string]any{"option": "a", "why": claim},
		},
	}
}

func newF11(t *testing.T) *f11 {
	f := &f11{f09: newF09(t)}
	fyi := map[string]any{"summary": "ok", "kind": "fyi", "status": "done"}
	receipt := map[string]any{"runId": "run_01JCZK7Q8XG4M2B9V5T3W6RH01", "harness": "claude-code", "toolCalls": 0}
	f.script = []map[string]any{
		step11("jm_whoami", map[string]any{}),
		step11("jm_post", plus(fyi, "summary", nil)),
		step11("jm_post", plus(fyi, "summary", longHeadline)),
		step11("jm_post", plus(validAsk("Merge now or wait?"), "options", nil)),
		step11("jm_post", plus(fyi, "needs", validAsk("x")["needs"])),
		step11("jm_post", plus(fyi, "receipt", receipt)),
		step11("jm_post", validAsk("Merge the auth fix now or wait for review?")),
	}
	raw, _ := json.Marshal(f.script)
	script := filepath.Join(t.TempDir(), "script.json")
	if err := os.WriteFile(script, raw, 0o600); err != nil {
		t.Fatal(err)
	}
	f.record = filepath.Join(t.TempDir(), "agent.json")
	f.harness(f.record, "hold", "script="+script)
	return f
}

// answer is what an MCP tool call came back with: refused or not, and the pod's own error code or output.
type answer struct {
	Refused bool
	Code    string
	Out     struct {
		MessageID, DecisionID string
		Seq                   int
	}
}

func (f *f11) answerTo(i int) (answer, error) {
	var a answer
	var env struct {
		Result struct {
			IsError           bool
			StructuredContent json.RawMessage
		}
	}
	if i >= len(f.rec.Calls) || json.Unmarshal(f.rec.Calls[i].Reply, &env) != nil {
		return a, fmt.Errorf("call %d (%s) has no readable reply: %+v", i, f.script[i]["tool"], f.rec.Calls)
	}
	a.Refused = env.Result.IsError
	var e struct{ Code string }
	_ = json.Unmarshal(env.Result.StructuredContent, &e)
	a.Code = e.Code
	_ = json.Unmarshal(env.Result.StructuredContent, &a.Out)
	return a, nil
}

// refusedWith: call i was refused with code, over MCP.
func (f *f11) refusedWith(i int, code string) error {
	a, err := f.answerTo(i)
	if err != nil {
		return err
	}
	if !a.Refused || a.Code != code {
		return fmt.Errorf("MCP call %d (%s) got refused=%v code %q, want refused with %s; reply %s", i, f.script[i]["tool"], a.Refused, a.Code, code, f.rec.Calls[i].Reply)
	}
	return nil
}

// runs claims the run and waits for the scripted agent to finish its calls.
func (f *f11) runs() error {
	if err := f.claims(f.record); err != nil {
		return err
	}
	rec, err := f.recorded(f.record, func(r agentRec) bool { return len(r.Calls) == len(f.script) })
	f.rec = rec
	return err
}

// httpAs sends a plain HTTP request (no MCP) with the token and extra headers; it returns the status and the body.
func (f *f11) httpAs(tok string, body any, hdr map[string]string) (int, []byte) {
	raw, _ := json.Marshal(body)
	req, _ := http.NewRequest(http.MethodPost, f.srv.base()+"/v1/pods/"+f.pod+"/messages", bytes.NewReader(raw))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+tok)
	for k, v := range hdr {
		req.Header.Set(k, v)
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return -1, []byte(err.Error())
	}
	defer resp.Body.Close()
	out, _ := io.ReadAll(resp.Body)
	return resp.StatusCode, out
}

func codeOf(raw []byte) string {
	var e struct{ Code string }
	_ = json.Unmarshal(raw, &e)
	return e.Code
}

// postedMsg is a stored message as the feed returns it.
type postedMsg struct {
	ID, Kind, Status, Summary, RunID string
	Seq                              int
	Decision                         *struct {
		ID, State string
		Ruling    json.RawMessage
	}
	Receipt *struct {
		RunID, AgentID, ComputerID, Harness, StartedAt, PostedAt, WakeReason string
		ToolCalls                                                            int
	}
}

func (f *f11) stored() ([]postedMsg, error) {
	var out struct{ Messages []postedMsg }
	if r := f.c.do("GET", "/v1/pods/"+f.pod+"/feed?limit=50", f.owner.tok, nil, &out); !r.ok() {
		return nil, fmt.Errorf("GET feed: HTTP %d", r.Status)
	}
	return out.Messages, nil
}

func (f *f11) storedWith(summary string) (postedMsg, error) {
	all, err := f.stored()
	for _, m := range all {
		if m.Summary == summary {
			return m, err
		}
	}
	return postedMsg{}, fmt.Errorf("no stored message with the summary %q among %d", summary, len(all))
}

// Step 1: a post without summary gets summary_required; one over 120 characters gets invalid_input, over MCP and plain HTTP.
func (f *f11) headline() error {
	if err := f.refusedWith(1, "summary_required"); err != nil {
		return err
	}
	if err := f.refusedWith(2, "invalid_input"); err != nil {
		return err
	}
	st, raw := f.httpAs(f.run.RunToken, plus(map[string]any{"kind": "fyi", "status": "done"}, "summary", nil), nil)
	if st != 400 || codeOf(raw) != "summary_required" {
		return fmt.Errorf("plain HTTP post without summary: HTTP %d %s, want 400 summary_required", st, raw)
	}
	st, raw = f.httpAs(f.run.RunToken, map[string]any{"summary": longHeadline, "kind": "fyi", "status": "done"}, nil)
	if st != 400 || codeOf(raw) != "invalid_input" {
		return fmt.Errorf("plain HTTP 121-character summary: HTTP %d %s, want 400 invalid_input", st, raw)
	}
	if st, raw = f.httpAs(f.run.RunToken, map[string]any{"summary": exactHeadline, "kind": "fyi", "status": "done"}, nil); st != 200 {
		return fmt.Errorf("plain HTTP 120-character summary: HTTP %d %s, want 200", st, raw)
	}
	return nil
}

// Step 2: an ask without options, or a non-ask with needs, gets invalid_input; and nothing refused is stored.
func (f *f11) shapes() error {
	for _, i := range []int{3, 4} {
		if err := f.refusedWith(i, "invalid_input"); err != nil {
			return err
		}
	}
	for name, body := range map[string]map[string]any{
		"an ask without needs":   plus(validAsk("a"), "needs", nil),
		"an ask without context": plus(validAsk("b"), "context", nil),
		"a fyi with options":     plus(map[string]any{"summary": "c", "kind": "fyi", "status": "done"}, "options", validAsk("x")["options"]),
		"a result with context":  plus(map[string]any{"summary": "d", "kind": "result", "status": "done"}, "context", validAsk("x")["context"]),
	} {
		if st, raw := f.httpAs(f.run.RunToken, body, nil); st != 400 || codeOf(raw) != "invalid_input" {
			return fmt.Errorf("%s: HTTP %d %s, want 400 invalid_input", name, st, raw)
		}
	}
	all, err := f.stored()
	for _, m := range all {
		if m.Summary != "Merge the auth fix now or wait for review?" && m.Summary != exactHeadline {
			return fmt.Errorf("a refused post was stored: %+v (err %v)", m, err)
		}
	}
	return err
}

// Step 3: a post carrying receipt is rejected over MCP and over plain HTTP, with a run token and with a user token.
func (f *f11) receiptRefused() error {
	if err := f.refusedWith(5, "invalid_input"); err != nil {
		return err
	}
	body := plus(validAsk("Receipt in the post"), "receipt", map[string]any{"runId": f.run.RunID, "harness": "codex", "toolCalls": 99})
	for who, tok := range map[string]string{"run": f.run.RunToken, "user": f.owner.tok} {
		if st, raw := f.httpAs(tok, body, nil); st != 400 || codeOf(raw) != "invalid_input" {
			return fmt.Errorf("plain HTTP post carrying receipt with a %s token: HTTP %d %s, want 400 invalid_input", who, st, raw)
		}
	}
	if _, err := f.storedWith("Receipt in the post"); err == nil {
		return fmt.Errorf("a post carrying a receipt was stored")
	}
	return nil
}

// Step 4: the valid ask is stored with the pod's receipt: the run, harness fake, why it woke, and the MCP calls so far.
func (f *f11) receipt() error {
	a, err := f.answerTo(askAt)
	if err != nil || a.Refused || !strings.HasPrefix(a.Out.MessageID, "msg_") {
		return fmt.Errorf("the valid ask over MCP: %+v, err %v; reply %s", a, err, f.rec.Calls[askAt].Reply)
	}
	m, err := f.storedWith("Merge the auth fix now or wait for review?")
	if err != nil {
		return err
	}
	r := m.Receipt
	if m.ID != a.Out.MessageID || m.Seq != a.Out.Seq || m.RunID != f.run.RunID || r == nil {
		return fmt.Errorf("stored message %+v does not match the post's answer %+v or carries no receipt", m, a.Out)
	}
	if r.RunID != f.run.RunID || r.AgentID != f.agentID || r.ComputerID != f.comp.ComputerID || r.Harness != "fake" || r.WakeReason != "task_assigned" || r.ToolCalls != askAt {
		return fmt.Errorf("receipt %+v, want run %s agent %s computer %s harness fake wakeReason task_assigned toolCalls %d", *r, f.run.RunID, f.agentID, f.comp.ComputerID, askAt)
	}
	if !stamp.MatchString(r.StartedAt) || !stamp.MatchString(r.PostedAt) || r.PostedAt < r.StartedAt {
		return fmt.Errorf("receipt times startedAt %q postedAt %q, want RFC 3339 with postedAt not before startedAt", r.StartedAt, r.PostedAt)
	}
	return f.agentCannotSteer(r.ToolCalls)
}

// agentCannotSteer: an agent that lies in headers changes nothing, and plain HTTP calls are not MCP calls, so the count stays.
func (f *f11) agentCannotSteer(before int) error {
	lies := map[string]string{"X-Jm-Tool-Calls": "99", "X-Jm-Harness": "codex", "X-Jm-Wake-Reason": "mention", "X-Jm-Run": "run_01JCZK7Q8XG4M2B9V5T3W6RH01"}
	st, raw := f.httpAs(f.run.RunToken, validAsk("The same ask over plain HTTP"), lies)
	if st != 200 {
		return fmt.Errorf("plain HTTP ask: HTTP %d %s, want 200", st, raw)
	}
	m, err := f.storedWith("The same ask over plain HTTP")
	if err != nil || m.Receipt == nil {
		return fmt.Errorf("the plain HTTP ask is not stored with a receipt: %+v %v", m, err)
	}
	if r := *m.Receipt; r.ToolCalls != before || r.Harness != "fake" || r.WakeReason != "task_assigned" || r.RunID != f.run.RunID {
		return fmt.Errorf("after headers claiming 99 calls, harness codex, wake mention: receipt %+v, want toolCalls %d harness fake wakeReason task_assigned", r, before)
	}
	return nil
}

// Step 5: the ask carries an open decision, and the post's answer names it.
func (f *f11) decision() error {
	a, _ := f.answerTo(askAt)
	m, err := f.storedWith("Merge the auth fix now or wait for review?")
	if err != nil {
		return err
	}
	d := m.Decision
	if d == nil || !decisionID.MatchString(d.ID) || d.State != "open" || len(d.Ruling) > 0 && string(d.Ruling) != "null" || d.ID != a.Out.DecisionID {
		return fmt.Errorf("the ask's decision is %+v, want an open dec_ id with no ruling, the one the post answered with (%q)", d, a.Out.DecisionID)
	}
	other, _ := f.storedWith("The same ask over plain HTTP")
	if other.Decision == nil || other.Decision.ID == d.ID {
		return fmt.Errorf("two asks share a decision: %+v and %+v", d, other.Decision)
	}
	if fyi, err := f.storedWith(exactHeadline); err != nil || fyi.Decision != nil {
		return fmt.Errorf("a fyi carries a decision: %+v %v", fyi, err)
	}
	return nil
}

// TestF11: jm_post enforces the headline contract and the pod stamps the receipt.
func TestF11(t *testing.T) {
	needFake(t)
	track(t)
	f := newF11(t)
	p := &stepper{out: os.Stdout, events: func() []Frame { return f.obs[0].snapshot(0) }}
	defer p.count(t)
	if err := f.runs(); err != nil {
		t.Fatalf("the scripted agent did not finish its MCP calls: %v", err)
	}
	p.step("a post without summary gets summary_required; one over 120 characters gets invalid_input", f.headline)
	p.step("an ask without options, or a non-ask with needs, gets invalid_input", f.shapes)
	p.step("a post carrying receipt is rejected", f.receiptRefused)
	p.step("a valid ask creates a message whose receipt has the runId, harness fake, wakeReason and toolCalls equal to the MCP calls made so far", f.receipt)
	p.step("the ask carries an open decision", f.decision)
	if p.ok != p.run {
		t.Fatalf("%d of %d steps passed", p.ok, p.run)
	}
}
