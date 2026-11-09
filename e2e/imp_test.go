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
	"sync"
	"sync/atomic"
	"syscall"
	"testing"

	"github.com/Team-Commonly/jellymoon/e2e/cmyfake"
)

// TestIMP: `jellymoon import commonly` against a fixture Commonly and a real local Jellymoon server (backlog unit IMP). Each step
// has its own fixture instance, config dir, work dir and recorder, on one shared Jellymoon server, so the steps run at the same
// time: an importer run is paced (at least 250 ms between two reads), so one after another they took over 20 s. The steps are
// plain subtests started from their own goroutines, not t.Parallel, so together they hold this test's one slot. The checks
// across everything still read every step's fixture, outputs, files and recorder once all of them have ended.
func TestIMP(t *testing.T) {
	need(t)
	par(t)
	track(t)
	bin, jm := buildDaemon(t), startServer(t)
	tok := mustUser(t, newClient(t, jm.base()), "importer").tok
	var (
		mu    sync.Mutex
		steps []*impEnv
		wg    sync.WaitGroup
	)
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
		wg.Add(1)
		go func() {
			defer wg.Done()
			t.Run(s.name, func(t *testing.T) {
				e := &impEnv{t: t, bin: bin, jm: jm, jmTok: tok, rec: newRecorder(t, jm.base())}
				e.jmURL = e.rec.URL
				defer func() { mu.Lock(); steps = append(steps, e); mu.Unlock() }() // kept even when the step stops on a Fatal
				s.fn(t, e)
				e.scanFiles()
			})
		}()
	}
	wg.Wait()
	impAcrossEverything(t, steps)
}

func impDryRun(t *testing.T, e *impEnv) {
	e.fresh(cmyfake.New())
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
	c := cmyfake.New()
	e.fresh(c)
	r := e.run("--apply")
	if r.code != 0 {
		t.Fatalf("exit %d\n%s\n%s", r.code, r.out, r.errOut)
	}
	for _, id := range []string{"p_dm", "p_foreign", "p_admin"} {
		if n := c.Seen("/api/messages/" + id); n != 0 {
			t.Errorf("%s was read %d times", id, n)
		}
	}
	if !strings.Contains(r.out, "dm-opt-in: 1") || !strings.Contains(r.out, "not-a-member: 1") {
		t.Errorf("the skips are not reported:\n%s", r.out)
	}
	t.Logf("SKIP outcome: exit %d\n%s", r.code, r.out)
}

func impRefusesAdmin(t *testing.T, e *impEnv) {
	c := cmyfake.New()
	c.Me["role"] = "admin"
	e.fresh(c)
	before := e.writes()
	r := e.run("--apply")
	if r.code == 0 || !strings.Contains(r.errOut, "admin") {
		t.Fatalf("exit %d\n%s\n%s", r.code, r.out, r.errOut)
	}
	if len(c.Requests()) != 1 || c.Seen("/api/pods") != 0 || e.writes() != before {
		t.Errorf("requests %v, writes %d -> %d", c.Requests(), before, e.writes())
	}
	t.Logf("REFUSE outcome: exit %d\n%s", r.code, r.errOut)
}

func impFailsOnSecret(t *testing.T, e *impEnv) {
	c := cmyfake.New()
	c.Msgs["p_study"][1]["content"] = "my key is cm_0123456789abcdef0123456789"
	e.fresh(c)
	before := e.writes()
	r := e.run("--apply")
	if r.code == 0 || !strings.Contains(r.errOut, "secret") || !strings.Contains(r.errOut, "p_study") || !strings.Contains(r.errOut, "s2") || strings.Contains(r.out+r.errOut, "0123456789abcdef") {
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
	e.fresh(cmyfake.New())
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
	e.fresh(cmyfake.New())
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
	c := cmyfake.New()
	c.FailMsg = func(pod string, call int) (int, string) {
		switch {
		case pod == "p_team" && call == 1:
			return 429, "1"
		case pod == "p_team" && call == 2:
			return 503, ""
		}
		return 0, ""
	}
	e.fresh(c)
	if r := e.run(); r.code != 0 || c.MaxInFlight() != 1 {
		t.Fatalf("exit %d, up to %d requests in flight\n%s\n%s", r.code, c.MaxInFlight(), r.out, r.errOut)
	}
}

func impOptIns(t *testing.T, e *impEnv) {
	c := cmyfake.New()
	e.fresh(c)
	r := e.run("--apply", "--include-dm", "p_dm", "--include-dm", "p_foreign", "--anonymize")
	if r.code != 0 || c.Seen("/api/messages/p_dm") == 0 || c.Seen("/api/messages/p_foreign") != 0 {
		t.Fatalf("exit %d, dm reads %d, foreign reads %d\n%s\n%s", r.code, c.Seen("/api/messages/p_dm"), c.Seen("/api/messages/p_foreign"), r.out, r.errOut)
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
	c := cmyfake.New()
	c.Grants = []cmyfake.Obj{{"grantId": "g1", "connector": cmyfake.Obj{"id": "linear"}, "writeMode": "read", "target": cmyfake.Obj{"kind": "pod", "id": "p_team"}, "token": "SECRETGRANTVALUE"}}
	c.Integ["p_team"] = []cmyfake.Obj{{"type": "discord", "status": "connected", "config": cmyfake.Obj{"botToken": "SECRETBOTVALUE", "webhookUrl": "https://hooks.example/SECRETHOOKVALUE"}}}
	e.fresh(c)
	if r := e.run("--apply"); r.code != 0 {
		t.Fatalf("exit %d\n%s\n%s", r.code, r.out, r.errOut)
	}
	list, _ := os.ReadFile(filepath.Join(e.work, "out", "grants-checklist.md"))
	for _, want := range []string{"linear", "discord"} {
		if !strings.Contains(string(list), want) {
			t.Errorf("the checklist lacks %q:\n%s", want, list)
		}
	}
}

// impAcrossEverything judges every step's fixture instances and everything recorded: no scope parameter, no forbidden route,
// only GETs with the run's token, and no secret in any output, file or request to Jellymoon.
func impAcrossEverything(t *testing.T, steps []*impEnv) {
	var cmys []*cmyfake.Server
	var all []string
	for _, e := range steps {
		cmys = append(cmys, e.cmys...)
		all = append(all, e.all...)
		all = append(all, e.rec.dump())
	}
	for i, c := range cmys {
		for _, bad := range []string{"scope", "/api/users/", "/api/credentials", "/api/agents/runtime", "ingest-tokens"} {
			if n := c.Seen(bad); n != 0 {
				t.Errorf("fixture %d saw %d requests with %q", i, n, bad)
			}
		}
		for _, r := range c.Requests() {
			if !strings.HasPrefix(r, "GET ") || !strings.HasSuffix(r, "auth=Bearer "+impSentinel) {
				t.Errorf("fixture %d: unexpected request %s", i, r)
			}
		}
	}
	scan := strings.Join(all, "\n")
	for _, bad := range []string{impSentinel, "SECRETGRANTVALUE", "SECRETBOTVALUE", "SECRETHOOKVALUE", "bob@example.com", "me@example.com"} {
		if strings.Contains(scan, bad) {
			t.Errorf("%q appears in an output, a file or a request to Jellymoon", bad)
		}
	}
}
