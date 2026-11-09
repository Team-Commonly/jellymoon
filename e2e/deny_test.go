package e2e

import (
	"bytes"
	"fmt"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// TestDENY: a signed-in person refuses a sign-in code they did not start (POST /v1/auth/device/deny, ADR 0008 step 4). The
// daemon's poll then gets access_denied and `jellymoon login` stops. A refusal of deny is approve's refusal for the same code,
// byte for byte. The last step moves the clock, so the steps run in order.
func TestDENY(t *testing.T) {
	needFake(t)
	par(t)
	track(t)
	srv := startServer(t)
	f := &f15{t: t, srv: srv, gh: gh, c: newClient(t, srv.base()), bin: buildDaemon(t), cfg: filepath.Join(t.TempDir(), "cfg"), out: &logBuf{}, done: make(chan error, 1)}
	b := &bsess{f15: f, own: srv.base()}
	if err := b.clock(fakeBase); err != nil {
		t.Fatal(err)
	}
	p := &stepper{out: os.Stdout, events: func() []Frame { return nil }}
	b.denySteps(p)
	p.count(t)
	if p.ok != p.run {
		t.Fatalf("%d of %d steps failed", p.run-p.ok, p.run)
	}
}

func (b *bsess) denySteps(p *stepper) {
	var hc *http.Client
	p.step("a person signs in through the stub GitHub", func() (err error) {
		hc, err = b.signedIn("octo-deny", 4400001)
		return
	})
	p.step("deny, then the poll is access_denied, again on the next poll; a second deny, in lower case without the hyphen, is still a success", func() error {
		return b.denyThenPoll(hc)
	})
	p.step("jellymoon login whose code is denied prints that the sign-in was refused in the browser and exits non-zero", func() error {
		return b.cliDenied(hc)
	})
	p.step("deny after approve is refused, and approve after deny, both with approve's used-code body", func() error {
		return b.denyApproveOrder(hc)
	})
	p.step("deny of an unknown or an exchanged code answers approve's body for the same code, byte for byte", func() error {
		return b.denyRefusals(hc)
	})
	p.step("deny has approve's rate limit: the start limit spent from this address stops neither", func() error {
		return b.denyLimit(hc)
	})
	p.step("deny with no session, a bad Bearer, no JSON content type, a foreign Origin or none is refused, and the code stays pending", func() error {
		return b.denyGuards(hc)
	})
	p.step("deny of an expired code answers approve's expired body, byte for byte (moves the clock: last)", func() error {
		return b.denyExpired(hc)
	})
}

// deny is POST /v1/auth/device/deny with the server's own Origin unless hdr says otherwise (an empty value drops a header).
func (b *bsess) deny(hc *http.Client, userCode string, hdr map[string]string) (int, string, []byte) {
	return b.authRoute(hc, "/v1/auth/device/deny", userCode, hdr)
}

// approveBody is approve with its raw body, to compare with deny's.
func (b *bsess) approveBody(hc *http.Client, userCode string) (int, string, []byte) {
	return b.authRoute(hc, "/v1/auth/device/approve", userCode, nil)
}

func (b *bsess) authRoute(hc *http.Client, path, userCode string, hdr map[string]string) (int, string, []byte) {
	h := map[string]string{"Content-Type": "application/json", "Origin": b.own}
	for k, v := range hdr {
		h[k] = v
		if v == "" {
			delete(h, k)
		}
	}
	return b.raw(hc, "POST", path, fmt.Sprintf(`{"userCode":%q}`, userCode), h)
}

func (b *bsess) denyThenPoll(hc *http.Client) error {
	d, err := b.start()
	if err != nil {
		return err
	}
	if st, code, body := b.deny(hc, d.UserCode, nil); st != 200 || string(body) != "{}" {
		return fmt.Errorf("deny a pending code: HTTP %d %q %s, want 200 {}", st, code, body)
	}
	for _, what := range []string{"the poll after deny", "the next poll"} {
		st, code, tok := b.poll(d.DeviceCode)
		if err := want(what, st, code, 400, "access_denied"); err != nil || tok.Token != "" {
			return fmt.Errorf("%v (token %q)", err, tok.Token)
		}
	}
	again := strings.ToLower(strings.ReplaceAll(d.UserCode, "-", ""))
	if st, code, body := b.deny(hc, again, nil); st != 200 || string(body) != "{}" {
		return fmt.Errorf("deny %q again: HTTP %d %q %s, want 200 {}", again, st, code, body)
	}
	return nil
}

// cliDenied runs the built daemon's login and denies the code it prints.
func (b *bsess) cliDenied(hc *http.Client) error {
	code, err := b.login()
	if err != nil {
		return err
	}
	if st, ec, _ := b.deny(hc, code, nil); st != 200 {
		return fmt.Errorf("deny the CLI's code: HTTP %d %q", st, ec)
	}
	select {
	case err := <-b.done:
		out := string(b.out.bytes())
		if err == nil || !strings.Contains(out, "jellymoon: the sign-in was refused in the browser\n") {
			return fmt.Errorf("jellymoon login after deny: exit %v, output %q; want a non-zero exit and the refused line", err, out)
		}
	case <-time.After(20 * time.Second):
		return fmt.Errorf("jellymoon login did not stop 20 s after deny; output %q", b.out.bytes())
	}
	if _, err := os.Stat(filepath.Join(b.cfg, "user-token")); !os.IsNotExist(err) {
		return fmt.Errorf("a denied login wrote a user-token (%v)", err)
	}
	return nil
}

func (b *bsess) denyApproveOrder(hc *http.Client) error {
	used, err := b.start()
	if err != nil {
		return err
	}
	if st, code := b.approve(hc, used.UserCode, nil); st != 200 {
		return fmt.Errorf("approve: HTTP %d %q", st, code)
	}
	wst, wcode, wbody := b.approveBody(hc, used.UserCode)
	if wst != 410 || wcode != "gone" {
		return fmt.Errorf("approve twice: HTTP %d %q, want 410 gone", wst, wcode)
	}
	if st, _, body := b.deny(hc, used.UserCode, nil); st != wst || !bytes.Equal(body, wbody) {
		return fmt.Errorf("deny after approve: HTTP %d %s, want approve's HTTP %d %s", st, body, wst, wbody)
	}
	denied, err := b.start()
	if err != nil {
		return err
	}
	if st, code, _ := b.deny(hc, denied.UserCode, nil); st != 200 {
		return fmt.Errorf("deny: HTTP %d %q", st, code)
	}
	if st, _, body := b.approveBody(hc, denied.UserCode); st != wst || !bytes.Equal(body, wbody) {
		return fmt.Errorf("approve after deny: HTTP %d %s, want the used-code HTTP %d %s", st, body, wst, wbody)
	}
	st, code, _ := b.poll(denied.DeviceCode)
	return want("the poll of a denied code someone then tried to approve", st, code, 400, "access_denied")
}

// sameRefusal checks that deny answers exactly what approve answers for the same code, and that approve refused it.
func (b *bsess) sameRefusal(hc *http.Client, what, userCode string, wst int, wcode string) error {
	ast, acode, abody := b.approveBody(hc, userCode)
	if ast != wst || acode != wcode {
		return fmt.Errorf("approve %s: HTTP %d %q, want %d %q", what, ast, acode, wst, wcode)
	}
	if st, _, body := b.deny(hc, userCode, nil); st != ast || !bytes.Equal(body, abody) {
		return fmt.Errorf("deny %s: HTTP %d %s, want approve's HTTP %d %s", what, st, body, ast, abody)
	}
	return nil
}

func (b *bsess) denyRefusals(hc *http.Client) error {
	if err := b.sameRefusal(hc, "an unknown code", "BBBB-BBBB", 404, "not_found"); err != nil {
		return err
	}
	d, err := b.start()
	if err != nil {
		return err
	}
	if st, code := b.approve(hc, d.UserCode, nil); st != 200 {
		return fmt.Errorf("approve: HTTP %d %q", st, code)
	}
	if st, code, tok := b.poll(d.DeviceCode); st != 200 || !strings.HasPrefix(tok.Token, "jmu_") {
		return fmt.Errorf("exchange: HTTP %d %q", st, code)
	}
	return b.sameRefusal(hc, "an exchanged code", d.UserCode, 410, "gone")
}

func (b *bsess) denyLimit(hc *http.Client) (err error) {
	window := b.now() + 60000 // a minute no request has touched
	if err := b.clock(window); err != nil {
		return err
	}
	defer func() {
		if e := b.clock(window + 60000); err == nil {
			err = e
		}
	}() // the next minute, so the later steps can start codes
	x, err := b.start()
	if err != nil {
		return err
	}
	y, err := b.start()
	if err != nil {
		return err
	}
	if got, err := b.limited("POST", "/v1/auth/device"); err != nil || got.status != 429 {
		return fmt.Errorf("spend the start limit: %+v %v, want 429", got, err)
	}
	for i := 0; i < 40; i++ {
		if st, code, _ := b.deny(hc, x.UserCode, nil); st != 200 {
			return fmt.Errorf("deny %d with the start limit spent: HTTP %d %q, want 200 like approve", i+1, st, code)
		}
	}
	if st, code := b.approve(hc, y.UserCode, nil); st != 200 {
		return fmt.Errorf("approve with the start limit spent: HTTP %d %q", st, code)
	}
	return nil
}

func (b *bsess) denyGuards(hc *http.Client) error {
	d, err := b.start()
	if err != nil {
		return err
	}
	for _, c := range []struct {
		hc   *http.Client
		hdr  map[string]string
		code string
	}{
		{browser(), nil, "unauthorized"},
		{browser(), map[string]string{"Authorization": "Bearer jmu_notarealtoken"}, "unauthorized"},
		{hc, map[string]string{"Content-Type": "text/plain"}, "forbidden"},
		{hc, map[string]string{"Origin": "https://evil.example"}, "forbidden"},
		{hc, map[string]string{"Origin": "null"}, "forbidden"},
		{hc, map[string]string{"Origin": ""}, "forbidden"},
	} {
		if st, code, _ := b.deny(c.hc, d.UserCode, c.hdr); code != c.code || (st != 401 && st != 403) {
			return fmt.Errorf("deny with %v: HTTP %d %q, want %q", c.hdr, st, code, c.code)
		}
	}
	st, code, _ := b.poll(d.DeviceCode)
	return want("the code after refused denies", st, code, 400, "authorization_pending")
}

func (b *bsess) denyExpired(hc *http.Client) error {
	d, err := b.start()
	if err != nil {
		return err
	}
	if err := b.clock(b.now() + int64(d.ExpiresIn+1)*1000); err != nil {
		return err
	}
	if err := b.sameRefusal(hc, "an expired code", d.UserCode, 400, "expired_token"); err != nil {
		return err
	}
	st, code, _ := b.poll(d.DeviceCode)
	return want("the poll of an expired code someone tried to deny", st, code, 400, "expired_token")
}
