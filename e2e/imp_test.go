package e2e

import (
	"bytes"
	"net/http"
	"net/http/httptest"
	"net/http/httputil"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"sync/atomic"
	"syscall"
	"testing"
)

// TestIMP: `jellymoon import commonly` against a fixture Commonly and a real local Jellymoon server (backlog unit IMP). Each step
// has its own fixture instance and config dir, on one shared Jellymoon server behind a recorder.
func TestIMP(t *testing.T) {
	need(t)
	par(t)
	track(t)
	env := &impEnv{t: t, bin: buildDaemon(t), jm: startServer(t)}
	env.jmURL = env.jm.base()
	env.jmTok = mustUser(t, newClient(t, env.jmURL), "importer").tok
	env.rec = newRecorder(t, env.jmURL)
	env.jmURL = env.rec.URL
	for _, s := range []struct {
		name string
		fn   func(*testing.T, *impEnv)
	}{
		{"dry-run reads, counts, and writes nothing anywhere", impDryRun},
		{"(a) a DM is skipped, and an agent-dm the person is not in is never read", impSkipsDMs},
		{"(b) an admin token is refused before any pod read", impRefusesAdmin},
		{"(c) a planted secret in a copied field fails the run with nothing written", impFailsOnSecret},
		{"apply twice posts nothing the second time", impIdempotent},
		{"a kill in the middle of a pod resumes with no gap and no duplicate", impKillResume},
		{"a 429 then a 503 are waited out, one request at a time", impBackoff},
		{"--include-dm opts one DM in, and --anonymize hides people", impOptIns},
		{"grants and integrations never leak a token, a bot token or a webhook url", impNoLeaks},
	} {
		t.Run(s.name, func(t *testing.T) {
			env.t = t
			s.fn(t, env)
			env.scanFiles()
		})
	}
	env.t = t
	env.acrossEverything()
}

func impDryRun(t *testing.T, e *impEnv) {
	e.fresh(newCmy())
	r := e.run()
	if r.code != 0 || !strings.Contains(r.out, "dry run") || !strings.Contains(r.out, "pods: 2") {
		t.Fatalf("dry run: exit %d\n%s\n%s", r.code, r.out, r.errOut)
	}
	if len(e.rec.hitsAll()) != 0 {
		t.Errorf("a dry run reached Jellymoon: %v", e.rec.hitsAll())
	}
	if _, err := os.Stat(filepath.Join(e.home, "import-commonly.json")); err == nil {
		t.Error("a dry run saved a state file")
	}
}

func impSkipsDMs(t *testing.T, e *impEnv) {
	c := newCmy()
	e.fresh(c)
	r := e.run("--apply")
	if r.code != 0 {
		t.Fatalf("exit %d\n%s\n%s", r.code, r.out, r.errOut)
	}
	for _, id := range []string{"p_dm", "p_foreign", "p_admin"} {
		if n := c.seen("/api/messages/" + id); n != 0 {
			t.Errorf("%s was read %d times", id, n)
		}
	}
	if !strings.Contains(r.out, "dm-opt-in: 1") || !strings.Contains(r.out, "not-a-member: 1") {
		t.Errorf("the skips are not reported:\n%s", r.out)
	}
	t.Logf("SKIP outcome: exit %d\n%s", r.code, r.out)
}

func impRefusesAdmin(t *testing.T, e *impEnv) {
	c := newCmy()
	c.me["role"] = "admin"
	e.fresh(c)
	before := e.writes()
	r := e.run("--apply")
	if r.code == 0 || !strings.Contains(r.errOut, "admin") {
		t.Fatalf("exit %d\n%s\n%s", r.code, r.out, r.errOut)
	}
	if len(c.reqs) != 1 || c.seen("/api/pods") != 0 || e.writes() != before {
		t.Errorf("requests %v, writes %d -> %d", c.reqs, before, e.writes())
	}
	t.Logf("REFUSE outcome: exit %d\n%s", r.code, r.errOut)
}

func impFailsOnSecret(t *testing.T, e *impEnv) {
	c := newCmy()
	c.msgs["p_study"][1]["content"] = "my key is cm_0123456789abcdef0123456789"
	e.fresh(c)
	before := e.writes()
	r := e.run("--apply")
	if r.code == 0 || !strings.Contains(r.errOut, "secret") || strings.Contains(r.out+r.errOut, "0123456789abcdef") {
		t.Fatalf("exit %d\n%s\n%s", r.code, r.out, r.errOut)
	}
	if e.writes() != before {
		t.Errorf("%d writes reached Jellymoon", e.writes()-before)
	}
	if _, err := os.Stat(filepath.Join(e.home, "import-commonly.json")); err == nil {
		t.Error("a failed run saved state")
	}
	t.Logf("FAIL outcome: exit %d\n%s", r.code, r.errOut)
}

func impIdempotent(t *testing.T, e *impEnv) {
	e.fresh(newCmy())
	if r := e.run("--apply"); r.code != 0 {
		t.Fatalf("first: exit %d\n%s\n%s", r.code, r.out, r.errOut)
	}
	after := e.writes()
	if r := e.run("--apply"); r.code != 0 {
		t.Fatalf("second: exit %d\n%s\n%s", r.code, r.out, r.errOut)
	}
	if e.writes() != after {
		t.Errorf("the second run wrote %d times", e.writes()-after)
	}
	m := e.markers()
	if len(m) != 5 {
		t.Errorf("markers %v", m)
	}
	for k, n := range m {
		if n != 1 {
			t.Errorf("%s imported %d times", k, n)
		}
	}
}

// impKillResume kills the importer with SIGKILL the moment Jellymoon has answered its second message post, then runs it again.
func impKillResume(t *testing.T, e *impEnv) {
	e.fresh(newCmy())
	var proc atomic.Pointer[os.Process]
	var posts atomic.Int32
	target, _ := url.Parse(e.rec.URL)
	killer := httptest.NewServer(&httputil.ReverseProxy{
		Rewrite: func(pr *httputil.ProxyRequest) { pr.SetURL(target) },
		ModifyResponse: func(resp *http.Response) error {
			if resp.Request.Method == "POST" && strings.HasSuffix(resp.Request.URL.Path, "/messages") && posts.Add(1) == 2 {
				if p := proc.Load(); p != nil {
					_ = syscall.Kill(p.Pid, syscall.SIGKILL)
				}
			}
			return nil
		},
	})
	defer killer.Close()
	saved := e.jmURL
	e.jmURL = killer.URL // both runs use it: the state file belongs to one server
	defer func() { e.jmURL = saved }()
	cmd := e.cmd("--apply")
	var so, se bytes.Buffer
	cmd.Stdout, cmd.Stderr = &so, &se
	if err := cmd.Start(); err != nil {
		t.Fatal(err)
	}
	proc.Store(cmd.Process)
	if first := e.finish(cmd, &so, &se, cmd.Wait()); first.code == 0 {
		t.Fatalf("the importer survived the kill:\n%s", first.out)
	}
	if r := e.run("--apply"); r.code != 0 {
		t.Fatalf("resume: exit %d\n%s\n%s", r.code, r.out, r.errOut)
	}
	e.jmURL = saved
	m := e.markers()
	for _, id := range []string{"m1", "m2", "m3", "s1", "s2"} {
		if m["commonly-msg:"+id] != 1 {
			t.Errorf("%s imported %d times (%v)", id, m["commonly-msg:"+id], m)
		}
	}
}

func impBackoff(t *testing.T, e *impEnv) {
	c := newCmy()
	c.failMsg = func(pod string, call int) (int, string) {
		switch {
		case pod == "p_team" && call == 1:
			return 429, "1"
		case pod == "p_team" && call == 2:
			return 503, ""
		}
		return 0, ""
	}
	e.fresh(c)
	if r := e.run(); r.code != 0 || c.max.Load() != 1 {
		t.Fatalf("exit %d, up to %d requests in flight\n%s\n%s", r.code, c.max.Load(), r.out, r.errOut)
	}
}

func impOptIns(t *testing.T, e *impEnv) {
	c := newCmy()
	e.fresh(c)
	r := e.run("--apply", "--include-dm", "p_dm", "--include-dm", "p_foreign", "--anonymize")
	if r.code != 0 || c.seen("/api/messages/p_dm") == 0 || c.seen("/api/messages/p_foreign") != 0 {
		t.Fatalf("exit %d, dm reads %d, foreign reads %d\n%s\n%s", r.code, c.seen("/api/messages/p_dm"), c.seen("/api/messages/p_foreign"), r.out, r.errOut)
	}
	var all []string
	for _, jm := range e.jmPods() {
		all = append(all, e.feed(jm)...)
	}
	joined := strings.Join(all, "\n")
	if strings.Contains(joined, "Bob Builder") || strings.Contains(joined, "Me Myself") || !strings.Contains(joined, "participant A") {
		t.Errorf("anonymize:\n%s", joined)
	}
}

func impNoLeaks(t *testing.T, e *impEnv) {
	c := newCmy()
	c.grants = []obj{{"grantId": "g1", "connector": obj{"id": "linear"}, "writeMode": "read", "target": obj{"kind": "pod", "id": "p_team"}, "token": "SECRETGRANTVALUE"}}
	c.integ["p_team"] = []obj{{"type": "discord", "status": "connected", "config": obj{"botToken": "SECRETBOTVALUE", "webhookUrl": "https://hooks.example/SECRETHOOKVALUE"}}}
	e.fresh(c)
	if r := e.run("--apply"); r.code != 0 {
		t.Fatalf("exit %d\n%s\n%s", r.code, r.out, r.errOut)
	}
}

// acrossEverything judges every fixture instance and everything recorded: no scope parameter, no forbidden route, only GETs
// with the run's token, and no secret in any output, file or request to Jellymoon.
func (e *impEnv) acrossEverything() {
	t := e.t
	for i, c := range e.cmys {
		for _, bad := range []string{"scope", "/api/users/", "/api/credentials", "/api/agents/runtime", "ingest-tokens"} {
			if n := c.seen(bad); n != 0 {
				t.Errorf("fixture %d saw %d requests with %q", i, n, bad)
			}
		}
		for _, r := range c.reqs {
			if !strings.HasPrefix(r, "GET ") || !strings.HasSuffix(r, "auth=Bearer "+impSentinel) {
				t.Errorf("fixture %d: unexpected request %s", i, r)
			}
		}
	}
	scan := strings.Join(e.all, "\n") + e.rec.dump()
	for _, bad := range []string{impSentinel, "SECRETGRANTVALUE", "SECRETBOTVALUE", "SECRETHOOKVALUE", "bob@example.test", "me@example.test"} {
		if strings.Contains(scan, bad) {
			t.Errorf("%q appears in an output, a file or a request to Jellymoon", bad)
		}
	}
}
