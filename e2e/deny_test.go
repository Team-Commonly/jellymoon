package e2e

import (
	"bytes"
	"encoding/json"
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
// byte for byte. Approve and deny share one guess limit, per person and in the address's start limit. The steps move the
// clock forward only, so they run in order.
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
	p.step("a person's 11th failed code lookup in ten minutes is rate_limited with Retry-After, on approve and on deny; successes do not count; past it even a valid code is refused, another person approves it, and the window resets", b.guessLimit)
	p.step("failed lookups count in the address's start limit: three accounts' 30 failures in one minute stop a fourth account and a start, until the minute ends", b.guessAddress)
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

// guess is approve or deny of userCode with the server's own Origin: the status, the error code and the Retry-After.
func (b *bsess) guess(hc *http.Client, route, userCode string) (int, string, string) {
	req, _ := http.NewRequest("POST", b.own+"/v1/auth/device/"+route, strings.NewReader(fmt.Sprintf(`{"userCode":%q}`, userCode)))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Origin", b.own)
	resp, err := hc.Do(req)
	if err != nil {
		return -1, err.Error(), ""
	}
	defer resp.Body.Close()
	var e ErrorBody
	_ = json.NewDecoder(resp.Body).Decode(&e)
	return resp.StatusCode, e.Code, resp.Header.Get("Retry-After")
}

// unknownCode is a code in the alphabet that no start minted (BBBB-BBBB is one too, for the steps before).
const unknownCode = "CCCC-CCCC"

func (b *bsess) guessLimit() error {
	g, err := b.signedIn("octo-guess", 4400002)
	if err != nil {
		return err
	}
	o, err := b.signedIn("octo-owner", 4400003)
	if err != nil {
		return err
	}
	window := (b.now()/600000 + 1) * 600000 // the start of a ten-minute window no request has touched
	if err := b.clock(window); err != nil {
		return err
	}
	x, err := b.start()
	if err != nil {
		return err
	}
	d, err := b.start()
	if err != nil {
		return err
	}
	for i := 0; i < 12; i++ {
		if st, code, _ := b.guess(g, "deny", x.UserCode); st != 200 {
			return fmt.Errorf("deny %d of a pending code: HTTP %d %q, want 200 (a success does not count)", i+1, st, code)
		}
	}
	for i, route := range []string{"approve", "deny", "approve", "deny", "approve", "deny", "approve", "deny", "approve", "deny"} {
		if st, code, _ := b.guess(g, route, unknownCode); st != 404 {
			return fmt.Errorf("failed lookup %d (%s): HTTP %d %q, want 404 not_found", i+1, route, st, code)
		}
	}
	for _, route := range []string{"approve", "deny"} {
		if st, code, retry := b.guess(g, route, unknownCode); st != 429 || code != "rate_limited" || retry != "600" {
			return fmt.Errorf("the 11th failed lookup by %s: HTTP %d %q Retry-After %q, want 429 rate_limited 600", route, st, code, retry)
		}
	}
	if st, code, _ := b.guess(g, "approve", d.UserCode); st != 429 || code != "rate_limited" {
		return fmt.Errorf("approve of a valid code past the limit: HTTP %d %q, want 429 rate_limited", st, code)
	}
	if st, code, _ := b.poll(d.DeviceCode); st != 400 || code != "authorization_pending" {
		return fmt.Errorf("the code a limited person tried: HTTP %d %q, want authorization_pending", st, code)
	}
	if st, code, _ := b.guess(o, "approve", d.UserCode); st != 200 {
		return fmt.Errorf("another person approves the code: HTTP %d %q, want 200", st, code)
	}
	if err := b.clock(window + 599000); err != nil {
		return err
	}
	if st, code, retry := b.guess(g, "deny", unknownCode); st != 429 || retry != "1" {
		return fmt.Errorf("the window's last second: HTTP %d %q Retry-After %q, want 429 with 1", st, code, retry)
	}
	if err := b.clock(window + 600000); err != nil {
		return err
	}
	st, code, _ := b.guess(g, "deny", unknownCode)
	return want("a failed lookup in the next window", st, code, 404, "not_found")
}

func (b *bsess) guessAddress() (err error) {
	var hcs []*http.Client
	for i := int64(0); i < 4; i++ {
		hc, err := b.signedIn(fmt.Sprintf("octo-addr-%d", i), 4400010+i)
		if err != nil {
			return err
		}
		hcs = append(hcs, hc)
	}
	minute := (b.now()/60000 + 1) * 60000 // a minute no request has touched
	if err := b.clock(minute); err != nil {
		return err
	}
	defer func() {
		if e := b.clock(minute + 60000); err == nil {
			err = e
		}
	}() // the next minute, so the later steps can start codes
	for i := 0; i < 31; i++ {
		st, code, retry := b.guess(hcs[i/10], "deny", unknownCode)
		if i < 30 && st != 404 {
			return fmt.Errorf("failed lookup %d, account %d: HTTP %d %q, want 404", i+1, i/10, st, code)
		}
		if i == 30 && (st != 429 || code != "rate_limited" || retry != "60") {
			return fmt.Errorf("the fourth account's first lookup after 30 from this address: HTTP %d %q Retry-After %q, want 429 rate_limited 60", st, code, retry)
		}
	}
	if st, code, _ := b.raw(browser(), "POST", "/v1/auth/device", "", nil); st != 429 || code != "rate_limited" {
		return fmt.Errorf("a start after 30 failed lookups from this address: HTTP %d %q, want 429 rate_limited", st, code)
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
