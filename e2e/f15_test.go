package e2e

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/cookiejar"
	"net/url"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
	"time"
)

type deviceStart struct {
	DeviceCode, UserCode, VerificationURI string
	Interval, ExpiresIn                   int
}

type deviceToken struct {
	Token string
	User  Actor
}

// f15 drives sign-in: the device flow over HTTP, a browser leg through the stub GitHub with a cookie jar, and `jellymoon login`.
type f15 struct {
	t    *testing.T
	srv  *server
	gh   *ghStub
	c    *client
	bin  string
	cfg  string
	out  *logBuf // the CLI's stdout and stderr
	done chan error
}

var userCodeRe = regexp.MustCompile(`[BCDFGHJKLMNPQRSTVWXZ]{4}-[BCDFGHJKLMNPQRSTVWXZ]{4}`)

// raw sends one request with the given headers and returns the status, the error code and the body.
func (f *f15) raw(hc *http.Client, method, path, body string, hdr map[string]string) (int, string, []byte) {
	req, _ := http.NewRequest(method, f.srv.base()+path, strings.NewReader(body))
	for k, v := range hdr {
		req.Header.Set(k, v)
	}
	resp, err := hc.Do(req)
	if err != nil {
		return -1, err.Error(), nil
	}
	defer resp.Body.Close()
	b, _ := io.ReadAll(resp.Body)
	var e ErrorBody
	_ = json.Unmarshal(b, &e)
	return resp.StatusCode, e.Code, b
}

func (f *f15) start() (deviceStart, error) {
	var d deviceStart
	if r := f.c.do("POST", "/v1/auth/device", "", nil, &d); !r.ok() {
		return d, fmt.Errorf("POST /v1/auth/device: HTTP %d %s", r.Status, r.Err.Code)
	}
	if len(d.DeviceCode) < 32 || !userCodeRe.MatchString(d.UserCode) || d.Interval < 1 || d.ExpiresIn < 1 {
		return d, fmt.Errorf("device start %+v breaks the schema", d)
	}
	return d, nil
}

func (f *f15) poll(code string) (int, string, deviceToken) {
	var out deviceToken
	r := f.c.do("POST", "/v1/auth/device/token", "", map[string]any{"deviceCode": code}, &out)
	return r.Status, r.Err.Code, out
}

// want checks a status and error code.
func want(what string, st int, code string, wst int, wcode string) error {
	if st != wst || code != wcode {
		return fmt.Errorf("%s: HTTP %d %q, want %d %q", what, st, code, wst, wcode)
	}
	return nil
}

func (f *f15) approve(hc *http.Client, userCode string, hdr map[string]string) (int, string) {
	h := map[string]string{"Content-Type": "application/json"}
	for k, v := range hdr {
		h[k] = v
	}
	st, code, _ := f.raw(hc, "POST", "/v1/auth/device/approve", fmt.Sprintf(`{"userCode":%q}`, userCode), h)
	return st, code
}

// browser is a cookie jar that does not follow redirects, so each hop is checked.
func browser() *http.Client {
	jar, _ := cookiejar.New(nil)
	return &http.Client{Jar: jar, Timeout: 10 * time.Second, CheckRedirect: func(*http.Request, []*http.Request) error { return http.ErrUseLastResponse }}
}

// authorize walks start -> stub authorize and returns the callback URL GitHub sends the browser back to.
func (f *f15) authorize(hc *http.Client, next string) (string, error) {
	resp, err := hc.Get(f.srv.base() + "/v1/auth/github/start?next=" + url.QueryEscape(next))
	if err != nil || resp.StatusCode != 302 || !strings.HasPrefix(resp.Header.Get("Location"), f.gh.srv.URL+"/login/oauth/authorize?") {
		return "", fmt.Errorf("start: %v %v, want 302 to the stub's authorize", err, resp)
	}
	resp, err = hc.Get(resp.Header.Get("Location"))
	if err != nil || resp.StatusCode != 302 {
		return "", fmt.Errorf("stub authorize: %v %v", err, resp)
	}
	return resp.Header.Get("Location"), nil
}

// signIn is authorize, then the callback: its status, Location and error code.
func (f *f15) signIn(hc *http.Client, next string) (st int, loc, code string, err error) {
	cb, err := f.authorize(hc, next)
	if err != nil {
		return 0, "", "", err
	}
	st, loc, code = f.callback(hc, cb)
	return st, loc, code, nil
}

func (f *f15) callback(hc *http.Client, cb string) (int, string, string) {
	resp, err := hc.Get(cb)
	if err != nil {
		return -1, "", err.Error()
	}
	defer resp.Body.Close()
	var e ErrorBody
	_ = json.NewDecoder(resp.Body).Decode(&e)
	return resp.StatusCode, resp.Header.Get("Location"), e.Code
}

// me is GET /v1/me with the browser's cookies, or with a bearer token when tok is set.
func (f *f15) me(hc *http.Client, tok string) (Actor, error) {
	h := map[string]string{}
	if tok != "" {
		h["Authorization"] = "Bearer " + tok
	}
	var a Actor
	st, code, b := f.raw(hc, "GET", "/v1/me", "", h)
	if st != 200 || json.Unmarshal(b, &a) != nil || a.Kind != "human" || !strings.HasPrefix(a.ID, "usr_") {
		return a, fmt.Errorf("GET /v1/me: HTTP %d %q %s", st, code, b)
	}
	return a, nil
}

func (f *f15) session(hc *http.Client) string {
	u, _ := url.Parse(f.srv.base() + "/")
	for _, ck := range hc.Jar.Cookies(u) {
		if ck.Name == "jm_session" {
			return ck.Value
		}
	}
	return ""
}

// login starts `jellymoon login` and waits for it to print the user code and the verification URL.
func (f *f15) login() (string, error) {
	cmd := exec.Command(f.bin, "login")
	cmd.Env = append(os.Environ(), "JELLYMOON_SERVER="+f.srv.base(), "JELLYMOON_HOME="+f.cfg)
	cmd.Stdout, cmd.Stderr = f.out, f.out
	if err := cmd.Start(); err != nil {
		return "", err
	}
	f.t.Cleanup(func() { _ = cmd.Process.Kill() })
	go func() { f.done <- cmd.Wait() }()
	for end := time.Now().Add(10 * time.Second); time.Now().Before(end); time.Sleep(50 * time.Millisecond) {
		o := string(f.out.bytes())
		if c := userCodeRe.FindString(o); c != "" && strings.Contains(o, f.srv.base()+"/device?code="+c) {
			return c, nil
		}
	}
	return "", fmt.Errorf("login printed no user code and URL: %q", f.out.bytes())
}

// TestF15: `jellymoon login` signs in through the server's device flow, with GitHub (a stub) proving who the person is.
func TestF15(t *testing.T) {
	need(t)
	par(t)
	track(t)
	f := &f15{t: t, gh: gh, srv: startServer(t), bin: buildDaemon(t), cfg: filepath.Join(t.TempDir(), "cfg"), out: &logBuf{}, done: make(chan error, 1)}
	f.c = newClient(t, f.srv.base())
	p := &stepper{out: os.Stdout, events: func() []Frame { return nil }}
	f.steps(p)
	p.count(t)
	if p.ok != p.run {
		t.Fatalf("%d of %d steps failed", p.run-p.ok, p.run)
	}
}

func (f *f15) steps(p *stepper) {
	var cliCode string
	var ana Actor
	b := browser()
	p.step("against a stub GitHub, jellymoon login prints the user code and verification URL", func() (err error) {
		cliCode, err = f.login()
		return
	})
	p.step("a device code polled before approval gets authorization_pending, and polled again at once slow_down", func() error {
		d, err := f.start()
		if err != nil {
			return err
		}
		st, code, _ := f.poll(d.DeviceCode)
		if err := want("first poll", st, code, 400, "authorization_pending"); err != nil {
			return err
		}
		st, code, _ = f.poll(d.DeviceCode)
		return want("second poll at once", st, code, 400, "slow_down")
	})
	p.step("the browser leg (start, stub GitHub, callback) sets the session cookie and lands on next; GET /v1/me returns the Actor", func() (err error) {
		st, loc, code, err := f.signIn(b, "/device?code="+cliCode)
		if err != nil {
			return err
		}
		if st != 302 || loc != "/device?code="+cliCode || f.session(b) == "" {
			return fmt.Errorf("callback: HTTP %d %q Location %q, session cookie %t; want 302 to the next path with a session", st, code, loc, f.session(b) != "")
		}
		if ana, err = f.me(b, ""); err == nil && ana.DisplayName != "octo-ana" {
			err = fmt.Errorf("GET /v1/me is %+v, want the GitHub login octo-ana", ana)
		}
		return err
	})
	p.step("approving the CLI's code with that session: login writes a 0600 token file, prints Signed in as octo-ana and never the token", func() error {
		return f.cliFinishes(b, cliCode, ana)
	})
	p.step("after approval exactly one exchange returns a new jmu_ for that user, the next invalid_grant; approving again is gone", func() error {
		return f.oneExchange(b, ana)
	})
	p.step("an unknown user code is not_found; approve without a JSON content type or with a foreign Origin is refused", func() error {
		return f.approveGuards(b)
	})
	p.step("the callback refuses a missing state, another browser's state and a reused state", func() error { return f.badStates() })
	p.step("an absolute, a // and a /\\ next land on /", func() error { return f.nexts() })
	p.step("a GitHub 401 on GET /user gets unauthorized; the same GitHub id twice is the same user, another id another", func() error {
		return f.identities(ana)
	})
	p.step("no GitHub token is in the server's data dir or its log", func() error { return f.noGitHubToken() })
	p.step("an expired code gets expired_token, on approve and on token (moves the clock: last)", func() error { return f.expired(b) })
}
