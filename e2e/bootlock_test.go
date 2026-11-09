package e2e

import (
	"net/http"
	"strings"
	"testing"
)

// TestBootLock starts two servers on one database (ADR 0010, "A rollout's overlap"). While the first holds the database's boot
// lock the second must exit before it answers anything, so it never restores a pod or serves a write. Once the first is gone
// (kill -9, which ends its session and so its lock), the same start succeeds and serves the first server's pod.
//
// It runs only when state lives in a real Postgres (JM_E2E_STATE_IN_DB=1, how CI runs `make e2e-c`): two servers given the same
// data dir share one schema there. On PGlite the data dir is one process's own database files, and on the Worker each process
// keeps its own state, so there is no shared database for a second process to meet.
func TestBootLock(t *testing.T) {
	need(t)
	if !stateInDatabase {
		t.Skip("needs a real Postgres shared by two processes (JM_E2E_STATE_IN_DB=1); see docs/development/e2e-contract.md, \"Runs on a real Postgres only\"")
	}
	track(t)
	par(t)
	first := startServer(t)
	c := newClient(t, first.base())
	owner, err := seedUser(c, "olivia")
	if err != nil {
		t.Fatalf("POST /__test/session: %v", err)
	}
	var pod struct {
		ID string `json:"id"`
	}
	if r := c.do("POST", "/v1/pods", owner.tok, map[string]any{"name": "owned"}, &pod); !r.ok() {
		t.Fatalf("create pod on the first server: HTTP %d %s", r.Status, r.Err.Code)
	}

	// The second server: same data dir, so the same schema; its own ports.
	second := &server{t: t, dir: first.dir, log: &logBuf{}}
	second.port = freePort(t, first.port, first.inspector)
	second.inspector = freePort(t, first.port, first.inspector, second.port)
	t.Cleanup(second.kill)
	if second.start() {
		t.Fatalf("a second server on the same database answered /healthz while the first held the boot lock")
	}
	if !strings.Contains(string(second.log.bytes()), "boot lock") {
		t.Fatalf("the second server exited without naming the boot lock; log tail:\n%s", second.log.tail(20))
	}
	// No write reaches it: it never listened.
	req, _ := http.NewRequest("POST", second.base()+"/v1/pods", strings.NewReader(`{"name":"intruder"}`))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+owner.tok)
	if resp, err := c.hc.Do(req); err == nil {
		resp.Body.Close()
		t.Fatalf("a write to the second server answered HTTP %d; it must never be served", resp.StatusCode)
	}

	first.kill()
	second.log = &logBuf{}
	second.boot()
	c2 := newClient(t, second.base())
	if r := c2.do("GET", "/v1/pods/"+pod.ID+"/me", owner.tok, nil, nil); r.Status != 200 {
		t.Fatalf("after the first server died, the second serves its pod: HTTP %d %s", r.Status, r.Err.Code)
	}
}
