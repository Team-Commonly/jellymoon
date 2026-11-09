package e2e

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"strings"
	"sync"
	"testing"
	"time"
)

const dayMs = int64(86400000)

// TestAUTHH: sign-in hardening (ADR 0008): a concurrent first sign-in, the user code in any case, logout with a Bearer and the
// cookie, logout closing the sockets of the revoked token, the start limit, user token expiry and the expiry sweep. The steps
// move the clock forward only, so their order is fixed.
func TestAUTHH(t *testing.T) {
	needFake(t)
	par(t)
	track(t)
	srv := startServer(t)
	a := &bsess{f15: &f15{t: t, srv: srv, gh: gh, c: newClient(t, srv.base())}, own: srv.base()}
	p := &stepper{out: os.Stdout, events: func() []Frame { return nil }}
	if err := a.clock(fakeBase); err != nil {
		t.Fatal(err)
	}
	a.authhSteps(p)
	p.count(t)
	if p.ok != p.run {
		t.Fatalf("%d of %d steps failed", p.run-p.ok, p.run)
	}
}

func (b *bsess) authhSteps(p *stepper) {
	p.step("two callbacks at once for the same new GitHub account both sign in, as one user", b.firstSignInRace)
	p.step("approve takes the user code in lower case and without its hyphen", b.codeCase)
	p.step("logout with a Bearer jmu_ and the cookie revokes both tokens and clears the cookie", b.logoutBoth)
	p.step("logout closes the pod socket opened with the revoked token and leaves the same person's other socket open", b.logoutCloses)
	p.step("each unauthenticated start is limited per address: rate_limited with Retry-After, and open again the next minute", b.startLimit)
	p.step("a user token past its expiresAt is unauthorized on every route and the socket, as Bearer and as cookie", b.tokenExpiry)
	p.step("an expired device code is swept an hour after expiry (invalid_grant) and a live one stays", b.sweep)
}

func (b *bsess) clock(now int64) error {
	if r := b.c.do("POST", "/__test/clock", "", map[string]any{"now": now}, nil); !r.ok() {
		return fmt.Errorf("POST /__test/clock: HTTP %d", r.Status)
	}
	return nil
}

func (b *bsess) now() int64 {
	var clk struct{ Now int64 }
	b.c.do("GET", "/__test/clock", "", nil, &clk)
	return clk.Now
}

// firstSignInRace signs one new GitHub account in from six browsers at once, three times over.
func (b *bsess) firstSignInRace() error {
	for i := int64(0); i < 3; i++ {
		id, login := 4300001+i, fmt.Sprintf("octo-race-%d", i)
		hcs, cbs, sts := make([]*http.Client, 6), make([]string, 6), make([]int, 6)
		for j := range hcs {
			var err error
			hcs[j] = browser()
			if cbs[j], err = b.stubCallback(hcs[j], login, id); err != nil {
				return err
			}
		}
		var wg sync.WaitGroup
		for j := range hcs {
			wg.Add(1)
			go func() { defer wg.Done(); sts[j], _, _ = b.callback(hcs[j], cbs[j]) }()
		}
		wg.Wait()
		users := map[string]bool{}
		for j, hc := range hcs {
			me, err := b.me(hc, "")
			if sts[j] != 302 || err != nil {
				return fmt.Errorf("concurrent callback %d for GitHub id %d: HTTP %d, then %v", j, id, sts[j], err)
			}
			users[me.ID] = true
		}
		if len(users) != 1 {
			return fmt.Errorf("one GitHub id %d signed in as %d users", id, len(users))
		}
	}
	return nil
}

func (b *bsess) codeCase() error {
	hc, err := b.signedIn("octo-authh-case", 4300050)
	if err != nil {
		return err
	}
	noHyphen := func(s string) string { return strings.ReplaceAll(s, "-", "") }
	for _, f := range []func(string) string{strings.ToLower, noHyphen, func(s string) string { return strings.ToLower(noHyphen(s)) }} {
		d, err := b.start()
		if err != nil {
			return err
		}
		if st, code := b.approve(hc, f(d.UserCode), nil); st != 200 {
			return fmt.Errorf("approve %q (shown as %s): HTTP %d %q, want 200", f(d.UserCode), d.UserCode, st, code)
		}
		if st, code, tok := b.poll(d.DeviceCode); st != 200 || !strings.HasPrefix(tok.Token, "jmu_") {
			return fmt.Errorf("exchange after approving %q: HTTP %d %q", f(d.UserCode), st, code)
		}
	}
	return nil
}

// status is GET path with only the given headers.
func (b *bsess) status(path string, hdr map[string]string) int {
	st, _, _ := b.raw(browser(), "GET", path, "", hdr)
	return st
}

func (b *bsess) logoutBoth() error {
	x, err := b.signedIn("octo-authh-both", 4300060)
	if err != nil {
		return err
	}
	y, err := b.signedIn("octo-authh-both", 4300060)
	if err != nil {
		return err
	}
	byCookie, byBearer := b.session(x), b.session(y)
	if st, code := b.send(x, "POST", "/v1/auth/logout", map[string]any{}, map[string]string{"Authorization": "Bearer " + byBearer}, nil); st != 200 || b.session(x) != "" {
		return fmt.Errorf("logout with both: HTTP %d %q, cookie still set %t", st, code, b.session(x) != "")
	}
	for what, tok := range map[string]string{"the cookie's token": byCookie, "the Bearer": byBearer} {
		if st := b.status("/v1/me", map[string]string{"Authorization": "Bearer " + tok}); st != 401 {
			return fmt.Errorf("%s after a logout that carried both: HTTP %d, want 401", what, st)
		}
	}
	return nil
}

// closed is true once s is closed, within d.
func closed(s *sock, d time.Duration) bool {
	for end := time.Now().Add(d); time.Now().Before(end); time.Sleep(50 * time.Millisecond) {
		if s.isDead() {
			return true
		}
	}
	return false
}

func (b *bsess) logoutCloses() error {
	x, err := b.signedIn("octo-authh-sock", 4300070)
	if err != nil {
		return err
	}
	y, err := b.signedIn("octo-authh-sock", 4300070)
	if err != nil {
		return err
	}
	var pod struct{ ID string }
	if st, code := b.send(x, "POST", "/v1/pods", map[string]any{"name": "authh"}, nil, &pod); st != 200 {
		return fmt.Errorf("POST /v1/pods by cookie: HTTP %d %q", st, code)
	}
	gone, err := b.dial(pod.ID, map[string]string{"Cookie": cookie(b.session(x)), "Origin": b.own})
	if err != nil {
		return err
	}
	kept, err := b.dial(pod.ID, map[string]string{"Authorization": "Bearer " + b.session(y)})
	if err != nil {
		return err
	}
	for _, s := range []*sock{gone, kept} {
		if _, err := firstHello(s); err != nil {
			return err
		}
	}
	if st, code := b.send(x, "POST", "/v1/auth/logout", map[string]any{}, nil, nil); st != 200 {
		return fmt.Errorf("logout by cookie: HTTP %d %q", st, code)
	}
	if !closed(gone, 5*time.Second) {
		return fmt.Errorf("the socket opened with the logged-out token is still open 5 s after logout")
	}
	if closed(kept, 500*time.Millisecond) {
		return fmt.Errorf("the socket opened with the person's other token closed too")
	}
	return nil
}

// refusal is the first answer to method path that is not a success, within 200 requests: how many got through, its status,
// its error code and its Retry-After.
type refusal struct {
	n, status   int
	code, retry string
}

// limited sends each request with a different X-Forwarded-For (and X-Real-IP), as a client may: the limit must not key on a
// header the caller writes, so it still refuses.
func (b *bsess) limited(method, path string) (refusal, error) {
	for n := 0; n < 200; n++ {
		req, _ := http.NewRequest(method, b.own+path, nil)
		req.Header.Set("X-Forwarded-For", fmt.Sprintf("203.0.113.%d", n%250+1))
		req.Header.Set("X-Real-IP", fmt.Sprintf("198.51.100.%d", n%250+1))
		resp, err := browser().Do(req)
		if err != nil {
			return refusal{n: n}, err
		}
		var e ErrorBody
		_ = json.NewDecoder(resp.Body).Decode(&e)
		resp.Body.Close()
		if resp.StatusCode != 200 && resp.StatusCode != 302 {
			return refusal{n, resp.StatusCode, e.Code, resp.Header.Get("Retry-After")}, nil
		}
	}
	return refusal{n: 200}, fmt.Errorf("%s %s: no limit within 200 requests", method, path)
}

func (b *bsess) startLimit() error {
	window := fakeBase + 60000 // a minute no request has touched
	if err := b.clock(window); err != nil {
		return err
	}
	for _, r := range [][2]string{{"POST", "/v1/auth/device"}, {"GET", "/v1/auth/github/start"}} {
		got, err := b.limited(r[0], r[1])
		if err != nil {
			return err
		}
		if got.status != 429 || got.code != "rate_limited" || got.retry != "60" || got.n < 5 {
			return fmt.Errorf("%s %s: %+v, want 429 rate_limited with Retry-After 60 after at least 5", r[0], r[1], got)
		}
	}
	if err := b.clock(window + 60000); err != nil {
		return err
	}
	if _, err := b.start(); err != nil {
		return fmt.Errorf("the next minute: %v", err)
	}
	if st := b.status("/v1/auth/github/start", nil); st != 302 {
		return fmt.Errorf("GitHub start the next minute: HTTP %d, want 302", st)
	}
	return nil
}

func (b *bsess) tokenExpiry() error {
	issued := b.now()
	x, err := b.signedIn("octo-authh-exp", 4300080)
	if err != nil {
		return err
	}
	var pod struct{ ID string }
	if st, code := b.send(x, "POST", "/v1/pods", map[string]any{"name": "expiry"}, nil, &pod); st != 200 {
		return fmt.Errorf("POST /v1/pods by cookie: HTTP %d %q", st, code)
	}
	tok := b.session(x)
	ways := map[string]map[string]string{"Bearer": {"Authorization": "Bearer " + tok}, "cookie": {"Cookie": cookie(tok), "Origin": b.own}}
	for _, at := range []struct {
		now  int64
		want int
	}{{issued + 30*dayMs - 1000, 200}, {issued + 30*dayMs, 401}} {
		if err := b.clock(at.now); err != nil {
			return err
		}
		for way, h := range ways {
			for _, path := range []string{"/v1/me", "/v1/pods/" + pod.ID + "/me"} {
				if st := b.status(path, h); st != at.want {
					return fmt.Errorf("GET %s by %s %d ms after issue: HTTP %d, want %d", path, way, at.now-issued, st, at.want)
				}
			}
			s, err := b.dial(pod.ID, h)
			if s != nil {
				s.close()
			}
			if (at.want == 200) != (err == nil) || (err != nil && !strings.Contains(err.Error(), "HTTP 401")) {
				return fmt.Errorf("the socket by %s %d ms after issue: %v, want %d", way, at.now-issued, err, at.want)
			}
		}
	}
	return nil
}

func (b *bsess) sweep() error {
	old, err := b.start()
	if err != nil {
		return err
	}
	if err := b.clock(b.now() + int64(old.ExpiresIn)*1000 + 3600000 + 1000); err != nil {
		return err
	}
	live, err := b.start()
	if err != nil {
		return err
	}
	st, code, _ := b.poll(old.DeviceCode)
	if err := want("a code swept an hour after its expiry", st, code, 400, "invalid_grant"); err != nil {
		return err
	}
	st, code, _ = b.poll(live.DeviceCode)
	return want("a live code after the sweep", st, code, 400, "authorization_pending")
}
