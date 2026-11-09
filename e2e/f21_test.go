package e2e

import (
	"fmt"
	"regexp"
	"slices"
	"strings"
	"testing"
)

// F21: the pod serves run evidence. Every MCP call a run makes is stamped as a tool-call record, and the evidence read returns them.
var toolCallID = regexp.MustCompile(`^tc_[0-9A-HJKMNP-TV-Z]{26}$`)

// toolCallKeys are the properties schema/message.json fixes for a ToolCallRecord; a record has no others.
var toolCallKeys = []string{"id", "runId", "source", "name", "status", "startedAt", "endedAt", "digest"}

type toolCall struct{ ID, RunID, Source, Name, Status, StartedAt, EndedAt, Digest string }

// evidence reads GET /v1/pods/:pod/runs/:run/tool-calls with the token, checking each record's shape on the way.
func (w *world) evidence(tok, runID string) ([]toolCall, res) {
	w.t.Helper()
	var raw struct{ ToolCalls []map[string]any }
	rs := w.c.do("GET", w.podPath("runs", runID, "tool-calls"), tok, nil, &raw)
	var out []toolCall
	for _, m := range raw.ToolCalls {
		for k := range m {
			if !slices.Contains(toolCallKeys, k) {
				w.t.Errorf("a record carries %q, which ToolCallRecord does not have", k)
			}
		}
		get := func(k string) string { s, _ := m[k].(string); return s }
		out = append(out, toolCall{get("id"), get("runId"), get("source"), get("name"), get("status"), get("startedAt"), get("endedAt"), get("digest")})
	}
	return out, rs
}

func (w *world) mustEvidence(tok, runID, what string) []toolCall {
	w.t.Helper()
	recs, rs := w.evidence(tok, runID)
	w.must(rs, what)
	return recs
}

// receiptCalls is the toolCalls of the receipt on a fyi the run posts over plain HTTP, which is not an MCP call and so not counted.
func (w *world) receiptCalls(r *run, summary string) int {
	w.t.Helper()
	w.must(w.runCall(r, "POST", w.podPath("messages"), post(map[string]any{"summary": summary}), nil), "post a fyi")
	var out struct {
		Messages []struct {
			Summary string
			Receipt *struct{ ToolCalls int }
		}
	}
	w.must(w.c.do("GET", w.podPath("feed")+"?limit=50", w.owner.tok, nil, &out), "read the feed")
	for _, m := range out.Messages {
		if m.Summary == summary && m.Receipt != nil {
			return m.Receipt.ToolCalls
		}
	}
	w.t.Fatalf("the fyi %q has no receipt in the feed", summary)
	return -1
}

// TestF21: three MCP calls leave three records in order; who may read them; nothing an agent sends writes one; the read caps at 500.
func TestF21(t *testing.T) {
	needFake(t)
	par(t)
	w := newWorld(t)
	r1, r2 := w.startRun(w.a1, "evidence"), w.startRun(w.a2, "other run")
	w.f21Stamped(r1)
	w.f21Readers(r1, r2)
	w.f21Unwritable(r1)
	w.f21Malformed(r1)
	w.f21Cap(r1)
}

// Steps 1 and 4 (count): a run makes three MCP calls; its token reads three records, in order, source pod, with a digest.
func (w *world) f21Stamped(r *run) {
	if got := w.mustEvidence(r.tok, r.id, "evidence before any MCP call"); len(got) != 0 {
		w.t.Errorf("before any MCP call the run has %d records, want 0", len(got))
	}
	names := []string{"jm_whoami", "jm_task_list", "jm_nope"}
	w.mcp(r.tok, names[0], `{}`)
	w.mcp(r.tok, names[1], `{}`)
	w.mcp(r.tok, names[2], `{}`)
	got := w.mustEvidence(r.tok, r.id, "evidence after three MCP calls")
	if len(got) != 3 {
		w.t.Fatalf("three MCP calls left %d records, want 3: %+v", len(got), got)
	}
	for i, c := range got {
		wantStatus := "ok"
		if i == 2 {
			wantStatus = "error" // a tool the pod does not have is a call that failed
		}
		if !toolCallID.MatchString(c.ID) || c.RunID != r.id || c.Source != "pod" || c.Name != names[i] || c.Status != wantStatus {
			w.t.Errorf("record %d is %+v, want a tc_ id, run %s, source pod, name %s, status %s", i, c, r.id, names[i], wantStatus)
		}
		if !stamp.MatchString(c.StartedAt) || !stamp.MatchString(c.EndedAt) || c.EndedAt < c.StartedAt || (i > 0 && c.StartedAt < got[i-1].StartedAt) {
			w.t.Errorf("record %d times %q to %q, want RFC 3339 and in order", i, c.StartedAt, c.EndedAt)
		}
		if c.Digest == "" || len([]rune(c.Digest)) > 600 || !strings.Contains(c.Digest, names[i]) {
			w.t.Errorf("record %d digest %q, want 1 to 600 characters starting from the call", i, c.Digest)
		}
	}
	if !strings.Contains(got[1].Digest, "tasks") {
		w.t.Errorf("the task_list digest %q does not hold the start of its output", got[1].Digest)
	}
	if n := w.receiptCalls(r, "count check"); n != len(got) {
		w.t.Errorf("the receipt counts %d tool calls, the read returned %d", n, len(got))
	}
}

// Step 2: a pod member's user token reads; a non-member, another run's token and another pod's token get forbidden.
func (w *world) f21Readers(r1, r2 *run) {
	want := w.mustEvidence(r1.tok, r1.id, "the run's own read")
	for name, tok := range map[string]string{"owner": w.owner.tok, "member": w.ben.tok, "other member": w.cara.tok} {
		if got := w.mustEvidence(tok, r1.id, name+" reads"); len(got) != len(want) || got[0].ID != want[0].ID {
			w.t.Errorf("%s reads %d records starting %v, want the run's own %d", name, len(got), got, len(want))
		}
	}
	stranger := w.session("stranger")
	_, rs := w.evidence(stranger.tok, r1.id)
	w.wantErr(rs, 403, "forbidden", "a user who is not a member")
	_, rs = w.evidence(r2.tok, r1.id)
	w.wantErr(rs, 403, "forbidden", "another run's token in the same pod")
	var b struct {
		ID string `json:"id"`
	}
	w.must(w.c.do("POST", "/v1/pods", w.owner.tok, map[string]any{"name": "pod-b"}, &b), "create pod B")
	w.wantErr(w.runCall(r1, "GET", "/v1/pods/"+b.ID+"/runs/"+r1.id+"/tool-calls", nil, nil), 403, "forbidden", "a run token on another pod")
	_, rs = w.evidence("", r1.id)
	w.wantErr(rs, 401, "unauthorized", "no token")
	_, rs = w.evidence(w.compA.tok, r1.id)
	w.wantErr(rs, 403, "forbidden", "a computer token")
	if got := w.mustEvidence(r1.tok, r1.id, "after the refusals"); len(got) != len(want) {
		w.t.Errorf("reading and being refused left %d records, want %d", len(got), len(want))
	}
}

// Step 3: a toolCalls or actor an agent sends is ignored, and no operation, method or body field writes a record.
func (w *world) f21Unwritable(r *run) {
	before := w.mustEvidence(r.tok, r.id, "before the forgeries")
	forged := `{"toolCalls":[{"id":"tc_01JCZK7Q8XG4M2B9V5T3W6RH01","name":"forged"}],"actor":{"kind":"user","id":"usr_x","displayName":"Olivia"},"source":"daemon"}`
	w.mcp(r.tok, "jm_whoami", forged)
	w.mcp(r.tok, "jm_evidence", `{"runId":"`+r.id+`","toolCalls":[{"name":"forged"}],"actor":{"kind":"user"}}`)
	after := w.mustEvidence(r.tok, r.id, "after the forgeries")
	if len(after) != len(before)+2 {
		w.t.Fatalf("two more MCP calls left %d records, want %d", len(after), len(before)+2)
	}
	for _, c := range after[len(before):] {
		if c.Source != "pod" || c.RunID != r.id || strings.Contains(c.Name, "forged") || c.ID == "tc_01JCZK7Q8XG4M2B9V5T3W6RH01" {
			w.t.Errorf("a forged field changed a record: %+v", c)
		}
	}
	path := w.podPath("runs", r.id, "tool-calls")
	for _, method := range []string{"POST", "PUT", "PATCH", "DELETE"} {
		for name, tok := range map[string]string{"run": r.tok, "owner": w.owner.tok} {
			if rs := w.c.do(method, path, tok, map[string]any{"toolCalls": []any{map[string]any{"name": "forged"}}, "name": "forged"}, nil); rs.ok() {
				w.t.Errorf("%s %s with a %s token: HTTP %d, want a refusal", method, path, name, rs.Status)
			}
		}
	}
	w.wantErr(w.runCall(r, "POST", w.podPath("messages"), post(map[string]any{"toolCalls": []string{"tc_01JCZK7Q8XG4M2B9V5T3W6RH01"}}), nil), 400, "invalid_input", "a post carrying toolCalls")
	if got := w.mustEvidence(r.tok, r.id, "after the writes"); len(got) != len(after) {
		w.t.Errorf("the refused writes left %d records, want %d", len(got), len(after))
	}
}

// Malformed and prototype-key input: ids that name no run, and tool names that are keys of Object.prototype or hold a NUL.
func (w *world) f21Malformed(r *run) {
	stranger := w.session("stranger-two")
	for _, id := range []string{"__proto__", "constructor", "toString", "nope", "run_00000000000000000000000000", "%00"} {
		_, rs := w.evidence(w.owner.tok, id)
		w.wantErr(rs, 404, "not_found", "a member reading run "+id)
		_, rs = w.evidence(stranger.tok, id)
		w.wantErr(rs, 403, "forbidden", "a non-member reading run "+id+" (forbidden before not_found)")
		_, rs = w.evidence(r.tok, id)
		w.wantErr(rs, 403, "forbidden", "a run token reading run "+id)
	}
	before := len(w.mustEvidence(r.tok, r.id, "before the odd names"))
	odd := []string{"__proto__", "constructor", "toString", "jm_\\u0000x", strings.Repeat("n", 90)}
	for _, name := range odd {
		w.mcp(r.tok, name, `{"__proto__":{"polluted":true},"constructor":1}`)
	}
	got := w.mustEvidence(r.tok, r.id, "after the odd names")
	if len(got) != before+len(odd) {
		w.t.Fatalf("%d odd tool calls left %d new records, want %d", len(odd), len(got)-before, len(odd))
	}
	for i, c := range got[before:] {
		if c.Status != "error" || c.Name == "" || len([]rune(c.Name)) > 64 || strings.ContainsRune(c.Name+c.Digest, 0) || len([]rune(c.Digest)) > 600 {
			w.t.Errorf("record for %q is %+v, want status error, a name of 1 to 64 characters and a clean digest of at most 600", odd[i], c)
		}
	}
	if got[before].Name != "__proto__" || got[before+1].Name != "constructor" {
		w.t.Errorf("a tool named like a prototype key was recorded as %q and %q", got[before].Name, got[before+1].Name)
	}
}

// Step 4: the read returns at most 500 records, the first 500 in order, while the receipt keeps counting.
func (w *world) f21Cap(r *run) {
	have := len(w.mustEvidence(r.tok, r.id, "before the cap"))
	for i := have; i < 505; i++ {
		if rep := w.mcp(r.tok, "jm_whoami", `{}`); rep.Status != 200 || rep.Refused {
			w.t.Fatalf("MCP call %d: HTTP %d refused=%v", i+1, rep.Status, rep.Refused)
		}
	}
	got := w.mustEvidence(r.tok, r.id, "at the cap")
	if len(got) != 500 {
		w.t.Fatalf("505 MCP calls: the read returned %d records, want 500", len(got))
	}
	seen := map[string]bool{}
	for i, c := range got {
		if seen[c.ID] || (i > 0 && c.StartedAt < got[i-1].StartedAt) {
			w.t.Fatalf("record %d (%s) repeats an id or is out of order", i, c.ID)
		}
		seen[c.ID] = true
	}
	if n := w.receiptCalls(r, "count past the cap"); n != 505 {
		w.t.Errorf("the receipt counts %d tool calls after 505 MCP calls, want 505 (%s)", n, fmt.Sprint(len(got)))
	}
}
