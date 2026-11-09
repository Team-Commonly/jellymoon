package e2e

import (
	"bufio"
	"crypto/rand"
	"crypto/sha1"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"net"
	"net/http"
	"net/url"
	"os"
	"strings"
	"testing"
	"time"
)

// bsess drives a browser that signed in through F15 (the jm_session cookie) against the pod's user routes and its socket
// (ADR 0008, "Browser session").
type bsess struct {
	*f15
	w   *world
	own string // the server's own origin
}

// signedIn is a browser signed in as a GitHub account, through the server's real start and callback. The stub's authorize
// hop is played here, with this test's own identity, so the parallel TestF15 steering the stub's next login cannot touch it.
func (b *bsess) signedIn(login string, id int64) (*http.Client, error) {
	hc := browser()
	resp, err := hc.Get(b.own + "/v1/auth/github/start?next=/")
	if err != nil || resp.StatusCode != 302 {
		return nil, fmt.Errorf("start: %v %v", err, resp)
	}
	q, _ := url.Parse(resp.Header.Get("Location"))
	code, tok := randHex(10), "gho_stub"+randHex(18)
	gh.mu.Lock()
	gh.codes[code], gh.tokens[tok], gh.users[tok] = tok, true, ghUser{ID: id, Login: login}
	gh.mu.Unlock()
	cb := q.Query().Get("redirect_uri") + "?" + url.Values{"code": {code}, "state": {q.Query().Get("state")}}.Encode()
	if st, _, ec := b.callback(hc, cb); st != 302 || b.session(hc) == "" {
		return nil, fmt.Errorf("callback as %s: HTTP %d %q, session %t", login, st, ec, b.session(hc) != "")
	}
	return hc, nil
}

// send is one request with the browser's cookies, a JSON content type and the server's own Origin unless hdr says otherwise
// (an empty value drops the header).
func (b *bsess) send(hc *http.Client, method, path string, body any, hdr map[string]string, out any) (int, string) {
	h := map[string]string{"Content-Type": "application/json", "Origin": b.own}
	for k, v := range hdr {
		h[k] = v
	}
	for k, v := range h {
		if v == "" {
			delete(h, k)
		}
	}
	j := []byte{}
	if body != nil {
		j, _ = json.Marshal(body)
	}
	st, code, raw := b.raw(hc, method, path, string(j), h)
	if out != nil {
		_ = json.Unmarshal(raw, out)
	}
	return st, code
}

// dial opens the pod socket with only the given headers: no Authorization unless hdr has one.
func (b *bsess) dial(pod string, hdr map[string]string) (*sock, error) {
	addr := strings.TrimPrefix(b.own, "http://")
	c, err := net.DialTimeout("tcp", addr, 5*time.Second)
	if err != nil {
		return nil, err
	}
	raw := make([]byte, 16)
	_, _ = rand.Read(raw)
	key := base64.StdEncoding.EncodeToString(raw)
	req, _ := http.NewRequest("GET", b.own+"/v1/pods/"+pod+"/ws", nil)
	for k, v := range map[string]string{"Upgrade": "websocket", "Connection": "Upgrade", "Sec-WebSocket-Key": key, "Sec-WebSocket-Version": "13"} {
		req.Header.Set(k, v)
	}
	for k, v := range hdr {
		req.Header.Set(k, v)
	}
	_ = c.SetDeadline(time.Now().Add(5 * time.Second))
	br := bufio.NewReader(c)
	var resp *http.Response
	if err = req.Write(c); err == nil {
		resp, err = http.ReadResponse(br, req)
	}
	sum := sha1.Sum([]byte(key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"))
	if err == nil && (resp.StatusCode != 101 || resp.Header.Get("Sec-WebSocket-Accept") != base64.StdEncoding.EncodeToString(sum[:])) {
		err = fmt.Errorf("upgrade refused: HTTP %d", resp.StatusCode)
	}
	if err != nil {
		c.Close()
		return nil, err
	}
	_ = c.SetDeadline(time.Time{})
	s := newSock("browser", pod, c, br)
	b.t.Cleanup(s.close)
	return s, nil
}

func cookie(tok string) string { return "jm_session=" + tok }

// TestBSESS: a browser signed in through F15 uses the pod with only its session cookie, under CSRF and Origin rules.
func TestBSESS(t *testing.T) {
	needFake(t)
	par(t)
	w := newWorld(t)
	b := &bsess{f15: &f15{t: t, srv: w.srv, gh: gh}, w: w, own: w.srv.base()}
	p := &stepper{out: os.Stdout, events: func() []Frame { return nil }}
	b.steps(p)
	p.count(t)
	if p.ok != p.run {
		t.Fatalf("%d of %d steps failed", p.run-p.ok, p.run)
	}
}

func (b *bsess) steps(p *stepper) {
	var ana, bo *http.Client
	var pod struct{ ID string }
	var s *sock
	p.step("two browsers sign in through the stub GitHub and hold only a session cookie", func() (err error) {
		if ana, err = b.signedIn("octo-bsess-ana", 4200001); err == nil {
			bo, err = b.signedIn("octo-bsess-bo", 4200002)
		}
		return
	})
	p.step("with the cookie alone: create a pod, create an invite, and accept it from the second browser", func() error {
		return b.podByCookie(ana, bo, &pod.ID)
	})
	p.step("the pod WebSocket opens with the cookie from the server's own origin and sends hello", func() (err error) {
		if s, err = b.dial(pod.ID, map[string]string{"Cookie": cookie(b.session(bo)), "Origin": b.own}); err == nil {
			var h helloData
			if h, err = firstHello(s); err == nil {
				s.send("subscribe", map[string]any{"sinceSeq": *h.HeadSeq})
			}
		}
		return
	})
	p.step("a message posted with the cookie fans out to the socket and reads back from the feed by cookie", func() error {
		return b.fanOut(ana, bo, s, pod.ID)
	})
	p.step("a cookie POST or PATCH without a JSON content type, from a foreign, look-alike or other-port Origin, or with no Origin is refused", func() error {
		return b.csrf(ana, pod.ID)
	})
	p.step("the pod WebSocket with the cookie from a foreign Origin, or none, is refused", func() error {
		for _, o := range []string{"https://evil.example", ""} {
			h := map[string]string{"Cookie": cookie(b.session(bo))}
			if o != "" {
				h["Origin"] = o
			}
			if _, err := b.dial(pod.ID, h); err == nil || !strings.Contains(err.Error(), "HTTP 403") {
				return fmt.Errorf("socket with Origin %q: %v, want upgrade refused with 403", o, err)
			}
		}
		return nil
	})
	p.step("a run token in the session cookie is no caller; Bearer callers need no Origin, and a Bearer wins over a cookie", func() error {
		return b.kinds(ana)
	})
	p.step("logout revokes the token (as cookie and as Bearer) and clears the cookie; a Bearer logout revokes that token", func() error {
		return b.logout(ana, bo)
	})
}

func (b *bsess) podByCookie(ana, bo *http.Client, pod *string) error {
	var made struct{ ID string }
	if st, code := b.send(ana, "POST", "/v1/pods", map[string]any{"name": "from a browser"}, nil, &made); st != 200 || !strings.HasPrefix(made.ID, "pod_") {
		return fmt.Errorf("POST /v1/pods by cookie: HTTP %d %q", st, code)
	}
	*pod = made.ID
	var inv struct{ Code string }
	if st, code := b.send(ana, "POST", "/v1/pods/"+made.ID+"/invites", map[string]any{}, nil, &inv); st != 200 || inv.Code == "" {
		return fmt.Errorf("POST invites by cookie: HTTP %d %q", st, code)
	}
	var acc struct{ Pod struct{ ID string } }
	if st, code := b.send(bo, "POST", "/v1/invites/"+inv.Code+"/accept", map[string]any{}, nil, &acc); st != 200 || acc.Pod.ID != made.ID {
		return fmt.Errorf("accept by cookie: HTTP %d %q pod %q", st, code, acc.Pod.ID)
	}
	return nil
}

func (b *bsess) fanOut(ana, bo *http.Client, s *sock, pod string) error {
	if s == nil {
		return fmt.Errorf("no socket")
	}
	mk := s.mark()
	if st, code := b.send(ana, "POST", "/v1/pods/"+pod+"/messages", post(map[string]any{"summary": "from the browser"}), nil, nil); st != 200 {
		return fmt.Errorf("POST messages by cookie: HTTP %d %q", st, code)
	}
	if _, ok := s.waitAfter(mk, isType("message.created"), 5*time.Second); !ok {
		return fmt.Errorf("no message.created on the cookie socket within 5 s")
	}
	var feed struct{ Messages []struct{ Summary string } }
	if st, code := b.send(bo, "GET", "/v1/pods/"+pod+"/feed", nil, map[string]string{"Origin": "", "Content-Type": ""}, &feed); st != 200 || len(feed.Messages) != 1 || feed.Messages[0].Summary != "from the browser" {
		return fmt.Errorf("GET feed by cookie: HTTP %d %q %+v", st, code, feed.Messages)
	}
	return nil
}

func (b *bsess) csrf(ana *http.Client, pod string) error {
	other := fmt.Sprintf("http://127.0.0.1:%d", b.w.srv.port%60000+1) // the same host on another port
	hs := []map[string]string{{"Content-Type": "text/plain"}, {"Origin": "https://evil.example"}, {"Origin": "null"}, {"Origin": ""}, {"Origin": b.own + ".evil.example"}, {"Origin": other}}
	for _, h := range hs {
		for _, c := range []struct {
			method, path string
			in           any
		}{{"POST", "/v1/pods", map[string]any{"name": "forged"}}, {"POST", "/v1/pods/" + pod + "/messages", post(map[string]any{"summary": "forged"})}, {"PATCH", "/v1/pods/" + pod + "/tasks/tsk_forged", map[string]any{"status": "done"}}} {
			if st, code := b.send(ana, c.method, c.path, c.in, h, nil); st != 403 || code != "forbidden" {
				return fmt.Errorf("cookie %s %s with %v: HTTP %d %q, want 403 forbidden", c.method, c.path, h, st, code)
			}
		}
	}
	var feed struct{ Messages []struct{ Summary string } }
	if st, _ := b.send(ana, "GET", "/v1/pods/"+pod+"/feed", nil, nil, &feed); st != 200 || len(feed.Messages) != 1 {
		return fmt.Errorf("after the refusals the feed holds %d messages (HTTP %d), want the one", len(feed.Messages), st)
	}
	return nil
}

func (b *bsess) kinds(ana *http.Client) error {
	r := b.w.startRun(b.w.a1, "a run token for a cookie")
	me := b.w.podPath("me")
	if st, code := b.send(browser(), "GET", me, nil, map[string]string{"Cookie": cookie(r.tok)}, nil); st != 401 {
		return fmt.Errorf("a run token as the cookie: HTTP %d %q, want 401", st, code)
	}
	if st, code := b.send(browser(), "GET", me, nil, map[string]string{"Authorization": "Bearer " + r.tok}, nil); st != 200 {
		return fmt.Errorf("the same run token as a Bearer: HTTP %d %q, want 200", st, code)
	}
	if res := b.w.c.do("POST", "/v1/pods", b.w.owner.tok, map[string]any{"name": "bearer"}, nil); !res.ok() {
		return fmt.Errorf("a Bearer POST with no Origin: HTTP %d %q", res.Status, res.Err.Code)
	}
	var made struct{ ID string }
	h := map[string]string{"Authorization": "Bearer " + b.w.owner.tok, "Origin": "https://evil.example", "Content-Type": "text/plain"}
	if st, code := b.send(ana, "POST", "/v1/pods", map[string]any{"name": "bearer wins"}, h, &made); st != 200 {
		return fmt.Errorf("a Bearer beside the cookie: HTTP %d %q, want 200 (the cookie is ignored)", st, code)
	}
	var who struct{ PodID string }
	if res := b.w.c.do("GET", "/v1/pods/"+made.ID+"/me", b.w.owner.tok, nil, &who); !res.ok() {
		return fmt.Errorf("the pod made with Bearer and cookie belongs to the Bearer: HTTP %d %q", res.Status, res.Err.Code)
	}
	return nil
}

func (b *bsess) logout(ana, bo *http.Client) error {
	tok := b.session(ana)
	if st, code := b.send(ana, "POST", "/v1/auth/logout", map[string]any{}, nil, nil); st != 200 || b.session(ana) != "" {
		return fmt.Errorf("logout by cookie: HTTP %d %q, cookie still set %t", st, code, b.session(ana) != "")
	}
	if st, _, _ := b.raw(browser(), "GET", "/v1/me", "", map[string]string{"Cookie": cookie(tok)}); st != 401 {
		return fmt.Errorf("the old cookie after logout: HTTP %d, want 401", st)
	}
	if st, _, _ := b.raw(browser(), "GET", "/v1/me", "", map[string]string{"Authorization": "Bearer " + tok}); st != 401 {
		return fmt.Errorf("the logged-out token as a Bearer: HTTP %d, want 401", st)
	}
	boTok := b.session(bo)
	if st, code, _ := b.raw(browser(), "POST", "/v1/auth/logout", "", map[string]string{"Authorization": "Bearer " + boTok}); st != 200 {
		return fmt.Errorf("logout by Bearer: HTTP %d %q", st, code)
	}
	if st, _, _ := b.raw(bo, "GET", "/v1/me", "", nil); st != 401 {
		return fmt.Errorf("the cookie of a token logged out by Bearer: HTTP %d, want 401", st)
	}
	return nil
}
