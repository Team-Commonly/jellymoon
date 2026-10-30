package e2e

import (
	"fmt"
	"os"
	"strings"
	"testing"
)

// TestF03: a seeded user creates a pod and gets hello as its first pod WebSocket frame.
func TestF03(t *testing.T) {
	need(t)
	track(t)
	srv := startServer(t)
	c := newClient(t, srv.base())
	var s *sock
	p := &stepper{out: os.Stdout, events: func() []Frame {
		if s == nil {
			return nil
		}
		return s.snapshot(0)
	}}
	var u user
	var podID string
	p.step("POST /__test/session (the contract's seed route) mints a user and returns a jmu_ token", func() (err error) {
		u, err = seedUser(c, "fran")
		return
	})
	p.step("POST /v1/pods with that token creates a pod and returns its id", func() error {
		var out struct {
			ID string `json:"id"`
		}
		if r := c.do("POST", "/v1/pods", u.tok, map[string]any{"name": "first pod"}, &out); !r.ok() || !strings.HasPrefix(out.ID, "pod_") {
			return fmt.Errorf("HTTP %d code=%q id=%q", r.Status, r.Err.Code, out.ID)
		}
		podID = out.ID
		return nil
	})
	p.step("GET /v1/pods/:pod/ws with the token upgrades and the first frame is hello with headSeq 0 and the user as owner", func() error {
		conn, br, err := wsDial(fmt.Sprintf("127.0.0.1:%d", srv.port), "/v1/pods/"+podID+"/ws", u.tok)
		if err != nil {
			return err
		}
		s = newSock("fran", podID, conn, br)
		t.Cleanup(s.close)
		h, err := firstHello(s)
		if err != nil {
			return err
		}
		if h.PodID != podID || *h.HeadSeq != 0 || h.You.ID != u.actor.ID {
			return fmt.Errorf("hello podId=%q headSeq=%d you=%q, want %s, 0, %s", h.PodID, *h.HeadSeq, h.You.ID, podID, u.actor.ID)
		}
		if len(h.Members) != 1 || h.Members[0].Actor.ID != u.actor.ID || h.Members[0].Role != "owner" {
			return fmt.Errorf("hello members %+v, want the user as the one owner", h.Members)
		}
		return nil
	})
	p.count(t)
	if p.ok != p.run {
		t.Fatalf("%d of %d steps passed", p.ok, p.run)
	}
}
