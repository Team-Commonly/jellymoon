package e2e

import (
	"fmt"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"time"
)

func (f *f15) cliFinishes(b *http.Client, cliCode string, ana Actor) error {
	if st, code := f.approve(b, cliCode, nil); st != 200 {
		return fmt.Errorf("approve the CLI's code: HTTP %d %q", st, code)
	}
	select {
	case err := <-f.done:
		if err != nil {
			return fmt.Errorf("jellymoon login: %v; output %q", err, f.out.bytes())
		}
	case <-time.After(20 * time.Second):
		return fmt.Errorf("jellymoon login did not finish 20 s after approval; output %q", f.out.bytes())
	}
	path := filepath.Join(f.cfg, "user-token")
	fi, err := os.Stat(path)
	if err != nil || fi.Mode().Perm() != 0o600 {
		return fmt.Errorf("token file %s: %v %v, want mode 0600", path, err, fi)
	}
	raw, _ := os.ReadFile(path)
	tok := strings.TrimSpace(string(raw))
	out := string(f.out.bytes())
	if !strings.HasPrefix(tok, "jmu_") || strings.Contains(out, tok) || strings.Contains(out, "jmu_") || !strings.Contains(out, "Signed in as octo-ana") {
		return fmt.Errorf("token file holds a jmu_ %t; the output %q must say Signed in as octo-ana and hold no token", strings.HasPrefix(tok, "jmu_"), out)
	}
	if tok == f.session(b) {
		return fmt.Errorf("the CLI got the browser's session token, want a new one")
	}
	a, err := f.me(f.c.hc, tok)
	if err == nil && a.ID != ana.ID {
		err = fmt.Errorf("the CLI's token is for %s, want the approver %s", a.ID, ana.ID)
	}
	return err
}

func (f *f15) oneExchange(b *http.Client, ana Actor) error {
	d, err := f.start()
	if err != nil {
		return err
	}
	if st, code := f.approve(b, d.UserCode, nil); st != 200 {
		return fmt.Errorf("approve with the server's own Origin: HTTP %d %q", st, code)
	}
	gap := time.Duration(d.Interval)*time.Second + 200*time.Millisecond
	time.Sleep(gap)
	st, code, tok := f.poll(d.DeviceCode)
	if st != 200 || !strings.HasPrefix(tok.Token, "jmu_") || tok.User.ID != ana.ID || tok.Token == f.session(b) {
		return fmt.Errorf("exchange: HTTP %d %q user %s, want 200, a new jmu_ for %s", st, code, tok.User.ID, ana.ID)
	}
	time.Sleep(gap)
	st, code, _ = f.poll(d.DeviceCode)
	if err := want("second exchange", st, code, 400, "invalid_grant"); err != nil {
		return err
	}
	st, code = f.approve(b, d.UserCode, nil)
	return want("approve twice", st, code, 410, "gone")
}

func (f *f15) approveGuards(b *http.Client) error {
	st, code := f.approve(b, "BBBB-BBBB", nil)
	if err := want("approve an unknown code", st, code, 404, "not_found"); err != nil {
		return err
	}
	d, err := f.start()
	if err != nil {
		return err
	}
	for _, h := range []map[string]string{{"Content-Type": "text/plain"}, {"Origin": "https://evil.example"}, {"Origin": "null"}, {"Origin": ""}} {
		if st, code := f.approve(b, d.UserCode, h); st != 403 || code != "forbidden" {
			return fmt.Errorf("approve with %v: HTTP %d %q, want 403 forbidden", h, st, code)
		}
	}
	if st, code := f.approve(f.c.hc, d.UserCode, nil); st != 401 {
		return fmt.Errorf("approve with no session: HTTP %d %q, want 401", st, code)
	}
	st, code, _ = f.poll(d.DeviceCode)
	return want("the refused code stays unapproved", st, code, 400, "authorization_pending")
}

func (f *f15) badStates() error {
	refused := func(what string, hc *http.Client, cb string) error {
		if st, _, code := f.callback(hc, cb); st != 403 || code != "forbidden" || f.session(hc) != "" {
			return fmt.Errorf("callback with %s: HTTP %d %q, session set %t; want 403 forbidden and no session", what, st, code, f.session(hc) != "")
		}
		return nil
	}
	b, other := browser(), browser()
	cb, err := f.authorize(b, "/bound")
	if err == nil {
		_, err = f.authorize(other, "/") // the other browser holds its own state cookie
	}
	if err != nil {
		return err
	}
	u, _ := url.Parse(cb)
	q := u.Query()
	state := q.Get("state")
	q.Del("state")
	u.RawQuery = q.Encode()
	if err := refused("a missing state", b, u.String()); err != nil {
		return err
	}
	if err := refused("another browser's state", other, cb); err != nil {
		return err
	}
	if st, loc, code := f.callback(b, cb); st != 302 || loc != "/bound" {
		return fmt.Errorf("the state in its own browser: HTTP %d %q Location %q, want 302 to the next bound at start", st, code, loc)
	}
	// Replay it with the state cookie put back, as a copy of this browser would; only the used state is left to refuse it.
	b.Jar.SetCookies(u, []*http.Cookie{{Name: "jm_session", Path: "/", MaxAge: -1}, {Name: "jm_oauth_state", Value: state, Path: "/v1/auth/github"}})
	return refused("a reused state", b, cb)
}

func (f *f15) nexts() error {
	for _, n := range []string{"https://evil.example/x", "//evil.example/x", `/\evil.example`, "javascript:alert(1)"} {
		if st, loc, code, err := f.signIn(browser(), n); err != nil || st != 302 || loc != "/" {
			return fmt.Errorf("next %q: %v HTTP %d %q Location %q, want 302 to /", n, err, st, code, loc)
		}
	}
	return nil
}

func (f *f15) identities(ana Actor) error {
	f.gh.as("octo-ana", 4100001, true)
	st, _, code, err := f.signIn(browser(), "/")
	f.gh.as("octo-ana", 4100001, false)
	if err != nil || st != 401 || code != "unauthorized" {
		return fmt.Errorf("a stub /user 401: %v HTTP %d %q, want 401 unauthorized", err, st, code)
	}
	for _, c := range []struct {
		login string
		id    int64
		same  bool
	}{{"octo-ana-renamed", 4100001, true}, {"octo-ben", 4100002, false}} {
		f.gh.as(c.login, c.id, false)
		b := browser()
		if st, _, code, err := f.signIn(b, "/"); err != nil || st != 302 {
			return fmt.Errorf("sign in as %s: %v HTTP %d %q", c.login, err, st, code)
		}
		a, err := f.me(b, "")
		if err != nil || (a.ID == ana.ID) != c.same {
			return fmt.Errorf("GitHub id %d signed in as %s (%v); same user as the first sign-in: %t, want %t", c.id, a.ID, err, a.ID == ana.ID, c.same)
		}
	}
	f.gh.as("octo-ana", 4100001, false)
	return nil
}

// noGitHubToken: the server dropped every GitHub token it was given. It reads the data dir's files and the server's log.
func (f *f15) noGitHubToken() error {
	toks := f.gh.issued()
	if len(toks) == 0 {
		return fmt.Errorf("the stub issued no GitHub token")
	}
	hit := ""
	_ = filepath.Walk(f.srv.dir, func(p string, fi os.FileInfo, err error) error {
		if err != nil || fi.IsDir() {
			return nil
		}
		raw, _ := os.ReadFile(p)
		for _, t := range toks {
			if strings.Contains(string(raw), t) {
				hit = p
			}
		}
		return nil
	})
	for _, t := range toks {
		if strings.Contains(string(f.srv.log.bytes()), t) {
			hit = "the server log"
		}
	}
	if hit != "" {
		return fmt.Errorf("a GitHub token is stored in %s", hit)
	}
	return nil
}

func (f *f15) expired(b *http.Client) error {
	var clk struct{ Now int64 }
	if r := f.c.do("GET", "/__test/clock", "", nil, &clk); !r.ok() {
		return fmt.Errorf("GET /__test/clock: HTTP %d", r.Status)
	}
	f.c.do("POST", "/__test/clock", "", map[string]any{"now": clk.Now}, nil)
	d, err := f.start()
	if err != nil {
		return err
	}
	if r := f.c.do("POST", "/__test/clock", "", map[string]any{"now": clk.Now + int64(d.ExpiresIn+1)*1000}, nil); !r.ok() {
		return fmt.Errorf("POST /__test/clock: HTTP %d", r.Status)
	}
	st, code := f.approve(b, d.UserCode, nil)
	if err := want("approve an expired code", st, code, 400, "expired_token"); err != nil {
		return err
	}
	st, code, _ = f.poll(d.DeviceCode)
	return want("exchange an expired code", st, code, 400, "expired_token")
}
