package e2e

import (
	"fmt"
	"os"
	"testing"
)

// memberRole is the role hello lists for id, or "" when hello does not list them.
func memberRole(h helloData, id string) string {
	for _, m := range h.Members {
		if m.Actor.ID == id {
			return m.Role
		}
	}
	return ""
}

// onlyMembers checks, through a fresh owner hello, that the pod's members are exactly the given users: a refused request left no one behind.
func (w *world) onlyMembers(want ...user) error {
	h, err := firstHello(w.dial(w.owner.tok, "olivia-check"))
	if err != nil {
		return err
	}
	if len(h.Members) != len(want) {
		return fmt.Errorf("hello lists %d members %+v, want exactly %d", len(h.Members), h.Members, len(want))
	}
	for _, u := range want {
		if memberRole(h, u.actor.ID) == "" {
			return fmt.Errorf("hello members %+v do not list %s", h.Members, u.actor.ID)
		}
	}
	return nil
}

// TestF05: invites add members; a used invite is refused with 410 and a non-member cannot invite.
func TestF05(t *testing.T) {
	needFake(t)
	w := newWorld(t)
	p := &stepper{out: os.Stdout, events: func() []Frame { return w.sO.snapshot(0) }}
	dana, eli, fay := w.session("dana"), w.session("eli"), w.session("fay")
	people := []user{w.owner, w.ben, w.cara, dana} // the pod after dana joins
	var code string
	p.step("the owner creates a single-use invite and a second user accepts it", func() error {
		var inv struct {
			Code string `json:"code"`
		}
		if r := w.c.do("POST", "/v1/pods/"+w.pod+"/invites", w.owner.tok, map[string]any{"maxUses": 1}, &inv); !r.ok() {
			return fmt.Errorf("create invite: HTTP %d code=%q", r.Status, r.Err.Code)
		}
		code = inv.Code
		if r := w.c.do("POST", "/v1/invites/"+code+"/accept", dana.tok, nil, nil); !r.ok() {
			return fmt.Errorf("accept invite: HTTP %d code=%q", r.Status, r.Err.Code)
		}
		return nil
	})
	p.step("the second user's hello lists them as a member", func() error {
		h, err := firstHello(w.dial(dana.tok, "dana"))
		if err == nil && memberRole(h, dana.actor.ID) != "member" {
			err = fmt.Errorf("hello members %+v do not list dana (%s) as a member", h.Members, dana.actor.ID)
		}
		return err
	})
	p.step("accepting the same single-use invite again returns 410", func() error {
		if r := w.c.do("POST", "/v1/invites/"+code+"/accept", eli.tok, nil, nil); r.Status != 410 {
			return fmt.Errorf("second accept: HTTP %d code=%q, want 410", r.Status, r.Err.Code)
		}
		return w.onlyMembers(people...) // the refused user did not join
	})
	p.step("accepting an expired invite returns 410 and adds no member", func() error {
		var inv struct {
			Code string `json:"code"`
		}
		if r := w.c.do("POST", "/v1/pods/"+w.pod+"/invites", w.owner.tok, map[string]any{"maxUses": 5, "ttlSeconds": 60}, &inv); !r.ok() {
			return fmt.Errorf("create invite: HTTP %d code=%q", r.Status, r.Err.Code)
		}
		w.advance(61_000) // past the invite's expiry, with uses left
		if r := w.c.do("POST", "/v1/invites/"+inv.Code+"/accept", fay.tok, nil, nil); r.Status != 410 {
			return fmt.Errorf("expired accept: HTTP %d code=%q, want 410", r.Status, r.Err.Code)
		}
		return w.onlyMembers(people...)
	})
	p.step("a non-member creating an invite gets 403", func() error {
		r := w.c.do("POST", "/v1/pods/"+w.pod+"/invites", eli.tok, map[string]any{"maxUses": 1}, nil)
		if r.Status != 403 || r.Err.Code != "forbidden" {
			return fmt.Errorf("non-member invite: HTTP %d code=%q, want 403 forbidden", r.Status, r.Err.Code)
		}
		return w.onlyMembers(people...)
	})
	p.count(t)
	if p.ok != p.run {
		t.Fatalf("%d of %d steps passed", p.ok, p.run)
	}
}
