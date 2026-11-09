package e2e

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"sort"
	"strings"
	"testing"
	"time"
)

// plainGet is a GET with no Content-Type and, when tok is empty, no token: what a signed-out browser sends.
func (w *world) plainGet(path, tok string) (int, []byte) {
	req, _ := http.NewRequest("GET", w.srv.base()+path, nil)
	if tok != "" {
		req.Header.Set("Authorization", "Bearer "+tok)
	}
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return -1, []byte(err.Error())
	}
	defer resp.Body.Close()
	b, _ := io.ReadAll(resp.Body)
	return resp.StatusCode, b
}

// helloNamesThePod: a member's socket gets a hello whose podName is the name the pod was created with.
func (w *world) helloNamesThePod() error {
	s := w.dial(w.ben.tok, "ben-hello")
	f, ok := s.waitAfter(0, isType("hello"), 5*time.Second)
	if !ok {
		return fmt.Errorf("no hello within 5 s")
	}
	var h struct {
		PodID   string `json:"podId"`
		PodName string `json:"podName"`
	}
	f.dec(&h)
	if h.PodID != w.pod || h.PodName != "contract" {
		return fmt.Errorf("hello is %s, want podId %s and podName \"contract\"", f.Data, w.pod)
	}
	return nil
}

type invite struct {
	Code      string `json:"code"`
	ExpiresAt string `json:"expiresAt"`
}

func (w *world) newInvite(body map[string]any) (invite, error) {
	var inv invite
	if r := w.c.do("POST", "/v1/pods/"+w.pod+"/invites", w.owner.tok, body, &inv); !r.ok() {
		return inv, fmt.Errorf("create invite: HTTP %d %s", r.Status, r.Err.Code)
	}
	return inv, nil
}

// readInvite is GET /v1/invites/:code signed out: 200 with exactly podName, inviterName and expiresAt.
func (w *world) readInvite(inv invite) error {
	st, b := w.plainGet("/v1/invites/"+inv.Code, "")
	if st != 200 {
		return fmt.Errorf("GET /v1/invites/:code: HTTP %d %s", st, b)
	}
	var got map[string]any
	_ = json.Unmarshal(b, &got)
	keys := make([]string, 0, len(got))
	for k := range got {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	if strings.Join(keys, ",") != "expiresAt,inviterName,podName" {
		return fmt.Errorf("the invite read carries %v, want exactly expiresAt, inviterName and podName: %s", keys, b)
	}
	if got["podName"] != "contract" || got["inviterName"] != w.owner.actor.DisplayName || got["expiresAt"] != inv.ExpiresAt {
		return fmt.Errorf("the invite read is %s, want pod \"contract\", inviter %q, expiry %s", b, w.owner.actor.DisplayName, inv.ExpiresAt)
	}
	return nil
}

// readingSpendsNothing: a one-use invite read three times keeps its expiry, and one person can still accept it.
func (w *world) readingSpendsNothing(inv invite) error {
	for i := 0; i < 3; i++ {
		if err := w.readInvite(inv); err != nil {
			return err
		}
	}
	dee := w.session("dee")
	if r := w.c.do("POST", "/v1/invites/"+inv.Code+"/accept", dee.tok, nil, nil); !r.ok() {
		return fmt.Errorf("accept after three reads: HTTP %d %s (a read used the invite up)", r.Status, r.Err.Code)
	}
	return nil
}

// sameNotFound: unknown, used and expired codes get one and the same 404 body.
func (w *world) sameNotFound(used invite) error {
	expiring, err := w.newInvite(map[string]any{"ttlSeconds": 60})
	if err != nil {
		return err
	}
	w.advance(61_000)
	unknown := "inv_" + strings.Repeat("A", 22)
	var first []byte
	for _, c := range []struct{ what, code string }{{"unknown", unknown}, {"used", used.Code}, {"expired", expiring.Code}} {
		st, b := w.plainGet("/v1/invites/"+c.code, "")
		if st != 404 {
			return fmt.Errorf("%s code: HTTP %d %s, want 404", c.what, st, b)
		}
		if first == nil {
			first = b
		} else if !bytes.Equal(b, first) {
			return fmt.Errorf("%s code: body %s differs from the unknown code's %s", c.what, b, first)
		}
	}
	return nil
}

// session: GET /v1/session is 200 signed out (no user), 200 signed in (the user), 200 with a dead token (no user); /v1/me stays 401.
func (w *world) session200() error {
	check := func(tok string, want *Actor) error {
		st, b := w.plainGet("/v1/session", tok)
		var got struct{ User *Actor }
		_ = json.Unmarshal(b, &got)
		if st != 200 || (want == nil) != (got.User == nil) || (want != nil && *got.User != *want) {
			return fmt.Errorf("GET /v1/session with %q: HTTP %d %s", tok, st, b)
		}
		return nil
	}
	if err := check("", nil); err != nil {
		return err
	}
	if err := check(w.owner.tok, &w.owner.actor); err != nil {
		return err
	}
	if err := check("jmu_"+strings.Repeat("x", 43), nil); err != nil {
		return err
	}
	if st, b := w.plainGet("/v1/me", ""); st != 401 {
		return fmt.Errorf("GET /v1/me signed out: HTTP %d %s, want 401", st, b)
	}
	return nil
}

// TestPODNAME: hello names the pod; a signed-out invite read shows the pod, the inviter and the expiry and spends nothing, and
// unknown, used and expired codes share one 404; GET /v1/session answers 200 signed in and out, while /v1/me keeps its 401.
func TestPODNAME(t *testing.T) {
	needFake(t)
	par(t)
	w := newWorld(t)
	p := &stepper{out: os.Stdout, events: func() []Frame { return w.sO.snapshot(0) }}
	p.step("a member's hello carries the pod's name", w.helloNamesThePod)
	inv, err := w.newInvite(map[string]any{"maxUses": 1})
	if err != nil {
		t.Fatal(err)
	}
	p.step("a signed-out read of an invite returns exactly the pod's name, the inviter's name and the expiry", func() error { return w.readInvite(inv) })
	p.step("reading a one-use invite three times leaves it whole: same expiry, and it can still be accepted", func() error { return w.readingSpendsNothing(inv) })
	p.step("unknown, used and expired codes get the same 404 body", func() error { return w.sameNotFound(inv) })
	p.step("GET /v1/session is 200 signed out, signed in and with a dead token; GET /v1/me signed out stays 401", w.session200)
	p.count(t)
	if p.ok != p.run {
		t.Fatalf("%d of %d steps passed", p.ok, p.run)
	}
}
