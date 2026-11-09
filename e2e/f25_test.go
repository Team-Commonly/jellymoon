package e2e

import (
	"bytes"
	"encoding/json"
	"io"
	"net/http"
	"strings"
	"testing"
)

// F25: an ask whose claims cite nothing, cite an id that is not a record here, or carry a number their evidence lacks is
// refused 422 ungrounded (docs/protocol.md, "Rules for Messages and cards", the two Grounding rules). The refusal names the
// claim by its place in the context and never echoes a digest or the claim's text.

// f25 is the evidence the asks cite: tool calls on runs of agent-one and agent-two, a message in this pod and one in another.
type f25 struct {
	*world
	r0, r1, r2   *run   // r0 an ended earlier run of a1, r1 a1's live run, r2 a2's run
	tc0, tc1     string // tc0 on r0 (its digest holds the task titles), tc1 on r1 (whoami, no titles)
	tc2          string // on r2: another agent's run
	msg, msgB    string // msg in this pod, msgB in another pod
	digests      []string
	refusedTexts []string
}

const (
	f25Title  = "Release 4.2 after 21274 of 21274 checks"
	f25Body   = "Coverage is 87.5% across 1,024 files; the delta was -3 lines."
	f25Absent = "tc_01JCZK7Q8XG4M2B9V5T3W6RH01"
)

// TestF25: steps (a) to (e) of the feature: cites nothing, not a record here, an ungrounded number, a fully cited ask, and what a
// refusal says.
func TestF25(t *testing.T) {
	needFake(t)
	par(t)
	g := newF25(newWorld(t))
	g.citesNothing()
	g.notARecord()
	g.numbers()
	g.accepted()
	g.nothingStored()
}

func newF25(w *world) *f25 {
	g := &f25{world: w}
	g.r0 = w.startRun(w.a1, f25Title)
	g.tc0 = g.oneCall(g.r0, "jm_task_list")
	w.must(w.c.do("POST", "/v1/runs/"+g.r0.id+"/end", w.compA.tok, map[string]any{"outcome": "completed"}, nil), "end the earlier run")
	g.r1 = w.startRun(w.a1, "Ship the fix")
	g.tc1 = g.oneCall(g.r1, "jm_whoami")
	g.r2 = w.startRun(w.a2, "Other work 99173")
	g.tc2 = g.oneCall(g.r2, "jm_task_list")
	for _, d := range g.digests[:1] {
		if !strings.Contains(d, "21274") || !strings.Contains(d, "4.2") {
			w.t.Fatalf("the task_list digest %q does not hold the task title, which the steps below cite", d)
		}
	}
	if strings.Contains(g.digests[1], "21274") {
		w.t.Fatalf("the whoami digest %q holds the title's number; the uncited-number step needs it not to", g.digests[1])
	}
	var m struct{ MessageID string }
	w.must(w.c.do("POST", w.podPath("messages"), w.owner.tok, map[string]any{"summary": "Coverage report", "kind": "fyi", "status": "done", "body": f25Body}, &m), "the owner posts the coverage report")
	g.msg = m.MessageID
	var b struct {
		ID string `json:"id"`
	}
	w.must(w.c.do("POST", "/v1/pods", w.owner.tok, map[string]any{"name": "pod-b"}, &b), "create pod B")
	w.must(w.c.do("POST", "/v1/pods/"+b.ID+"/messages", w.owner.tok, map[string]any{"summary": "21274 of 21274 checks passed", "kind": "fyi", "status": "done"}, &m), "post in pod B")
	g.msgB = m.MessageID
	return g
}

// oneCall makes one MCP call on the run's token and returns the id of the record the pod stamped for it.
func (g *f25) oneCall(r *run, tool string) string {
	g.t.Helper()
	before := len(g.mustEvidence(r.tok, r.id, "evidence before "+tool))
	g.mcp(r.tok, tool, `{}`)
	recs := g.mustEvidence(r.tok, r.id, "evidence after "+tool)
	if len(recs) != before+1 {
		g.t.Fatalf("%s left %d new records, want 1", tool, len(recs)-before)
	}
	g.digests = append(g.digests, recs[before].Digest)
	return recs[before].ID
}

func cites(text string, toolCalls, messages []string) map[string]any {
	c := map[string]any{"text": text}
	if toolCalls != nil {
		c["toolCalls"] = toolCalls
	}
	if messages != nil {
		c["messages"] = messages
	}
	return c
}

// ask is a valid ask whose four kinds of claim all cite tc1 unless `at` replaces one: "happened", "checked", "stakes" or "why".
func (g *f25) ask(summary string, at map[string]map[string]any) map[string]any {
	good := cites("The fix is on the branch.", []string{g.tc1}, nil)
	pick := func(k string) map[string]any {
		if c, ok := at[k]; ok {
			return c
		}
		return good
	}
	return map[string]any{
		"summary": summary, "kind": "ask", "status": "blocked",
		"needs": map[string]any{"to": map[string]any{"role": "owner"}, "action": "approve"},
		"options": []any{
			map[string]any{"id": "a", "label": "Ship now", "consequence": "The agent ships on its next run."},
			map[string]any{"id": "b", "label": "Wait", "consequence": "The agent waits for review."},
		},
		"context": map[string]any{
			"happened": []any{pick("happened")}, "checked": []any{pick("checked")}, "stakes": pick("stakes"),
			"recommendation": map[string]any{"option": "a", "why": pick("why")},
		},
	}
}

// postRaw posts with the token and returns the status and the raw body, so a refusal can be read for what it echoes.
func (g *f25) postRaw(tok string, body any) (int, []byte) {
	raw, _ := json.Marshal(body)
	req, _ := http.NewRequest("POST", g.srv.base()+g.podPath("messages"), bytes.NewReader(raw))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+tok)
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		g.t.Fatalf("POST messages: %v", err)
	}
	defer resp.Body.Close()
	out, _ := io.ReadAll(resp.Body)
	return resp.StatusCode, out
}

// refused: the ask is refused 422 ungrounded; the message names `where` and echoes neither the claim's text nor any digest (step e).
func (g *f25) refused(what, tok, where string, claim map[string]any, key string) {
	g.t.Helper()
	summary := "Refused: " + what
	if len(summary) > 120 {
		summary = summary[:120]
	}
	g.refusedTexts = append(g.refusedTexts, summary)
	st, raw := g.postRaw(tok, g.ask(summary, map[string]map[string]any{key: claim}))
	var e ErrorBody
	_ = json.Unmarshal(raw, &e)
	if st != 422 || e.Code != "ungrounded" {
		g.t.Errorf("%s: HTTP %d %s, want 422 ungrounded", what, st, raw)
		return
	}
	if !strings.Contains(e.Message, where) {
		g.t.Errorf("%s: the refusal %q does not name the claim %s", what, e.Message, where)
	}
	text, _ := claim["text"].(string)
	for _, leak := range append([]string{text, "21274", "87.5", "Release", "Coverage", "jm_task_list", "jm_whoami"}, g.digests...) {
		if leak != "" && bytes.Contains(raw, []byte(leak)) {
			g.t.Errorf("%s: the refusal %s echoes %q", what, raw, leak)
		}
	}
}

// Step (a): a claim with no toolCalls and no messages, or with both empty, is ungrounded, wherever it sits in the context.
func (g *f25) citesNothing() {
	tok := g.r1.tok
	g.refused("a happened claim citing nothing", tok, "context.happened[0]", cites("The fix is on the branch.", nil, nil), "happened")
	g.refused("a checked claim with empty citations", tok, "context.checked[0]", cites("The fix is on the branch.", []string{}, []string{}), "checked")
	g.refused("stakes citing nothing", tok, "context.stakes", cites("Waiting holds the release.", nil, nil), "stakes")
	g.refused("the recommendation citing nothing", tok, "context.recommendation.why", cites("Ship it.", nil, nil), "why")
	if r := g.mcp(tok, "jm_post", mustJSON(g.ask("Refused over MCP", map[string]map[string]any{"happened": cites("Nothing cited.", nil, nil)}))); !r.Refused || r.Code != "ungrounded" {
		g.t.Errorf("an ask citing nothing over MCP: refused=%v code %q, want refused ungrounded", r.Refused, r.Code)
	}
	g.refusedTexts = append(g.refusedTexts, "Refused over MCP")
}

// Step (b): an id that is no record in this pod, another agent's tool call, another pod's message, or a person citing a tool call.
func (g *f25) notARecord() {
	tok := g.r1.tok
	g.refused("a tool call id that is no record", tok, "context.happened[0]", cites("The fix is on the branch.", []string{f25Absent}, nil), "happened")
	g.refused("a message id that is no record", tok, "context.happened[0]", cites("The fix is on the branch.", nil, []string{"msg_01JCZK7Q8XG4M2B9V5T3W6RH01"}), "happened")
	g.refused("one good cite beside one bad", tok, "context.checked[0]", cites("The fix is on the branch.", []string{g.tc1, f25Absent}, nil), "checked")
	g.refused("another agent's tool call", tok, "context.happened[0]", cites("Other work went fine.", []string{g.tc2}, nil), "happened")
	g.refused("a message in another pod", tok, "context.stakes", cites("Checks passed.", nil, []string{g.msgB}), "stakes")
	g.refused("a person citing a tool call", g.owner.tok, "context.happened[0]", cites("The fix is on the branch.", []string{g.tc1}, nil), "happened")
}

// Step (c): every number in a claim's text must be in a digest it cites or the summary or body of a message it cites.
func (g *f25) numbers() {
	tok := g.r1.tok
	g.refused("a number in no evidence", tok, "context.happened[0]", cites("21274 of 21275 checks passed.", []string{g.tc0}, nil), "happened")
	g.refused("a number only in an uncited record", tok, "context.happened[0]", cites("21274 checks passed.", []string{g.tc1}, nil), "happened")
	g.refused("a number only in an uncited tool call", tok, "context.checked[0]", cites("Version 4.2 is out.", nil, []string{g.msg}), "checked")
	g.refused("a part of a decimal", tok, "context.checked[0]", cites("Coverage is 87 percent.", nil, []string{g.msg}), "checked")
	g.refused("trailing zero on a decimal", tok, "context.stakes", cites("Release 4.20 waits.", []string{g.tc0}, nil), "stakes")
	g.refused("a sign the evidence lacks", tok, "context.recommendation.why", cites("Coverage moved -87.5 points.", nil, []string{g.msg}), "why")
	g.refused("a number inside a word is still a number", tok, "context.happened[0]", cites("Build b98213 passed.", []string{g.tc1}, nil), "happened")
}

// Step (d): a fully cited ask is accepted over HTTP and MCP, citing an earlier run of the same agent and a message.
func (g *f25) accepted() {
	tok := g.r1.tok
	at := map[string]map[string]any{
		"happened": cites("Release 4.2 passed 21274 of 21274 checks.", []string{g.tc0}, nil),
		"checked":  cites("Coverage is 87.5% across 1024 files, 1,024 in all, and the delta was 3 lines.", nil, []string{g.msg}),
		"stakes":   cites("Waiting holds the release for review.", []string{g.tc1}, nil),
		"why":      cites("All 21274 checks passed and coverage is 87.5%.", []string{g.tc0, g.tc1}, []string{g.msg}),
	}
	var out struct{ MessageID, DecisionID string }
	g.must(g.runCall(g.r1, "POST", g.podPath("messages"), g.ask("Ship the fix now?", at), &out), "a fully cited ask")
	wantID(g.t, "dec_", out.DecisionID)
	if r := g.mcp(tok, "jm_post", mustJSON(g.ask("Ship the fix now, over MCP?", at))); r.Refused {
		g.t.Errorf("the same ask over MCP was refused: %+v", r)
	}
	person := map[string]map[string]any{"happened": cites("Coverage is 87.5%.", nil, []string{g.msg}), "checked": cites("Coverage is 87.5%.", nil, []string{g.msg}),
		"stakes": cites("Coverage is 87.5%.", nil, []string{g.msg}), "why": cites("Coverage is 87.5%.", nil, []string{g.msg})}
	g.must(g.c.do("POST", g.podPath("messages"), g.owner.tok, g.ask("A person asks, citing a message", person), nil), "a person's ask citing a message")
}

// No refused ask was stored.
func (g *f25) nothingStored() {
	var out struct{ Messages []struct{ Summary string } }
	g.must(g.c.do("GET", g.podPath("feed")+"?limit=50", g.owner.tok, nil, &out), "read the feed")
	for _, m := range out.Messages {
		for _, s := range g.refusedTexts {
			if m.Summary == s {
				g.t.Errorf("a refused ask was stored: %q", s)
			}
		}
	}
}

func mustJSON(v any) string {
	raw, _ := json.Marshal(v)
	return string(raw)
}
