package e2e

import (
	"strings"
	"testing"
)

// post is a valid PostInput; a case overrides or removes fields.
func post(over map[string]any) map[string]any {
	m := map[string]any{"summary": "all green", "kind": "fyi", "status": "done"}
	for k, v := range over {
		if v == nil {
			delete(m, k)
		} else {
			m[k] = v
		}
	}
	return m
}

// TestInputValidation: a write route refuses a body the schema refuses, with 400 invalid_input, and a body the schema allows
// is stored whole. Nothing is dropped or truncated (ADR 0003, "The adapter validates every Cmd against gen/").
func TestInputValidation(t *testing.T) {
	needFake(t)
	w := newWorld(t)
	w.refusedPosts()
	r := w.startRun(w.a1, "report")
	w.storedPosts(r)
	w.taskUpdates(r)
	w.otherRoutes(r)
}

func (w *world) refusedPosts() {
	refused := []struct {
		name string
		body map[string]any
	}{
		{"numeric body", post(map[string]any{"body": 5})},
		{"16,385-character body", post(map[string]any{"body": strings.Repeat("a", 16385)})},
		{"two-line summary", post(map[string]any{"summary": "first\nsecond"})},
		{"121-character summary", post(map[string]any{"summary": strings.Repeat("s", 121)})},
		{"missing kind", post(map[string]any{"kind": nil})},
		{"missing status", post(map[string]any{"status": nil})},
		{"unknown kind", post(map[string]any{"kind": "essay"})},
		{"unknown status", post(map[string]any{"status": "bogus"})},
		{"field only the server sets", post(map[string]any{"receipt": map[string]any{}})},
		// The core stores a post as fyi/done with none of these; refusing them keeps a post from being stored as something it is not.
		{"kind the core cannot store", post(map[string]any{"kind": "result"})},
		{"status the core cannot store", post(map[string]any{"status": "blocked"})},
		{"needs", post(map[string]any{"needs": map[string]any{"to": map[string]any{"role": "owner"}, "action": "review"}})},
		{"caveat", post(map[string]any{"caveat": "unchecked"})},
		{"links", post(map[string]any{"links": []any{}})},
	}
	for _, c := range refused {
		w.wantErr(w.c.do("POST", w.podPath("messages"), w.owner.tok, c.body, nil), 400, "invalid_input", "post with "+c.name)
	}
}

func (w *world) storedPosts(r *run) {
	t, msgs := w.t, w.podPath("messages")
	// The largest body the schema allows is stored whole, and a post by a person needs no more than the schema's three fields.
	whole := strings.Repeat("b", 16384)
	var posted struct {
		MessageID string `json:"messageId"`
		Seq       int64  `json:"seq"`
	}
	w.must(w.c.do("POST", msgs, w.owner.tok, post(map[string]any{"body": whole}), &posted), "post with a 16,384-character body")
	if got := w.feedBody(posted.MessageID); !strings.HasSuffix(got, whole) {
		t.Errorf("a 16,384-character body came back as %d characters, not whole", len(got))
	}

	// A run posts with its summary (it used to answer summary_required for every post).
	var byRun struct {
		MessageID string `json:"messageId"`
	}
	w.must(w.runCall(r, "POST", msgs, post(map[string]any{"summary": "run says hi"}), &byRun), "a run's post with a summary")
	if s := w.feedSummary(byRun.MessageID); s != "run says hi" {
		t.Errorf("a run's summary came back as %q, want %q", s, "run says hi")
	}
}

func (w *world) taskUpdates(r *run) {
	// A task update: the status is validated, and the schema's note, which the core has no place for, is refused rather than dropped.
	tk := w.newTask("update me", "")
	w.mustClaim(r, tk.ID)
	path := w.podPath("tasks", tk.ID)
	w.wantErr(w.runCall(r, "PATCH", path, map[string]any{"status": "in_progress", "note": "halfway"}, nil), 400, "invalid_input", "task update with a note")
	w.wantErr(w.runCall(r, "PATCH", path, map[string]any{"status": "open"}, nil), 400, "invalid_input", "task update to open")
	w.wantErr(w.runCall(r, "PATCH", path, map[string]any{"status": "in_progress", "taskId": tk.ID}, nil), 400, "invalid_input", "task update repeating the path id")
	w.must(w.runCall(r, "PATCH", path, map[string]any{"status": "in_progress"}, nil), "task update without a note")
}

func (w *world) otherRoutes(r *run) {
	tk := w.tasks()[0]
	w.wantErr(w.c.do("POST", "/v1/pods", w.owner.tok, map[string]any{"name": ""}, nil), 400, "invalid_input", "pod with an empty name")
	w.wantErr(w.c.do("POST", "/v1/pods", w.owner.tok, map[string]any{"name": "x", "extra": 1}, nil), 400, "invalid_input", "pod with an unknown field")
	w.wantErr(w.c.do("POST", w.podPath("invites"), w.owner.tok, map[string]any{"maxUses": 0}, nil), 400, "invalid_input", "invite with maxUses 0")
	w.wantErr(w.c.do("POST", w.podPath("invites"), w.owner.tok, map[string]any{"ttlSeconds": "soon"}, nil), 400, "invalid_input", "invite with a string ttl")
	w.wantErr(w.c.do("POST", "/v1/computers", w.owner.tok, map[string]any{"name": "m", "os": "plan9", "arch": "arm64"}, nil), 400, "invalid_input", "computer with an unknown os")
	w.wantErr(w.c.do("POST", w.podPath("agents"), w.compA.tok, map[string]any{"harness": "fake"}, nil), 400, "invalid_input", "agent without a displayName")
	w.wantErr(w.c.do("POST", "/v1/runs", w.compA.tok, map[string]any{"wakeId": 7}, nil), 400, "invalid_input", "run claim with a numeric wakeId")
	w.wantErr(w.c.do("POST", "/v1/runs/"+r.id+"/end", w.compA.tok, map[string]any{"outcome": "exploded"}, nil), 400, "invalid_input", "run end with an unknown outcome")
	w.wantErr(w.runCall(r, "POST", w.podPath("tasks", tk.ID, "release"), map[string]any{"reason": strings.Repeat("r", 161)}, nil), 400, "invalid_input", "release with a 161-character reason")
	w.wantErr(w.c.do("POST", w.podPath("runs", r.id, "stop"), w.ben.tok, map[string]any{"reason": 3}, nil), 400, "invalid_input", "stop with a numeric reason")
}

// feedMessages reads the pod's messages with bodies, as the owner.
func (w *world) feedMessages() []struct{ ID, Summary, Body string } {
	var out struct {
		Messages []struct{ ID, Summary, Body string } `json:"messages"`
	}
	w.must(w.c.do("GET", w.podPath("feed")+"?withBody=true&limit=50", w.owner.tok, nil, &out), "read the feed")
	return out.Messages
}

func (w *world) feedBody(id string) string {
	for _, m := range w.feedMessages() {
		if m.ID == id {
			return m.Body
		}
	}
	w.t.Fatalf("message %s is not in the feed", id)
	return ""
}

func (w *world) feedSummary(id string) string {
	for _, m := range w.feedMessages() {
		if m.ID == id {
			return m.Summary
		}
	}
	w.t.Fatalf("message %s is not in the feed", id)
	return ""
}
