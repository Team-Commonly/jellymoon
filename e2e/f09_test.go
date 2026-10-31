package e2e

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"net/http/httputil"
	"net/url"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"sync"
	"syscall"
	"testing"
	"time"
)

// hit is one plain HTTP request the daemon (or its agent) made to the server, seen by the recording proxy.
type hit struct {
	Method, Path, Auth string
	In, Out            []byte
	Status             int
}

// recorder sits between the daemon and the server under test. It forwards everything, upgrades included, and keeps the
// plain requests, so the suite can say what the daemon sent and what the server answered without reading any server log.
type recorder struct {
	*httptest.Server
	mu   sync.Mutex
	hits []hit
}

func newRecorder(t *testing.T, target string) *recorder {
	u, _ := url.Parse(target)
	r := &recorder{}
	rp := &httputil.ReverseProxy{Rewrite: func(pr *httputil.ProxyRequest) {
		pr.SetURL(u)
		pr.Out.Header.Del("Accept-Encoding") // keep bodies readable here
		pr.Out.Host = pr.In.Host             // the server builds the MCP URL from the host it was asked on, so it names the proxy
		if pr.Out.Header.Get("Upgrade") == "" {
			in, _ := io.ReadAll(pr.In.Body)
			pr.Out.Body = io.NopCloser(bytes.NewReader(in))
			pr.Out.GetBody = func() (io.ReadCloser, error) { return io.NopCloser(bytes.NewReader(in)), nil }
		}
	}}
	rp.ModifyResponse = func(resp *http.Response) error {
		if resp.StatusCode == http.StatusSwitchingProtocols {
			return nil
		}
		out, _ := io.ReadAll(resp.Body)
		resp.Body = io.NopCloser(bytes.NewReader(out))
		var in []byte
		if b, err := resp.Request.GetBody(); err == nil {
			in, _ = io.ReadAll(b)
		}
		r.mu.Lock()
		r.hits = append(r.hits, hit{resp.Request.Method, resp.Request.URL.Path, resp.Request.Header.Get("Authorization"), in, out, resp.StatusCode})
		r.mu.Unlock()
		return nil
	}
	r.Server = httptest.NewServer(rp)
	t.Cleanup(r.Close)
	return r
}

// find waits for a request whose method and path match, and returns it.
func (r *recorder) find(method string, path *regexp.Regexp, d time.Duration) (hit, bool) {
	for end := time.Now().Add(d); time.Now().Before(end); time.Sleep(25 * time.Millisecond) {
		r.mu.Lock()
		for _, h := range r.hits {
			if h.Method == method && path.MatchString(h.Path) {
				r.mu.Unlock()
				return h, true
			}
		}
		r.mu.Unlock()
	}
	return hit{}, false
}

// agentRec is what the fake agent wrote about itself (e2e/testdata/fakeagent).
type agentRec struct {
	PID         int
	Argv, Env   []string
	Initialize  json.RawMessage
	SessionNew  json.RawMessage
	Prompt      json.RawMessage
	Whoami      json.RawMessage
	WhoamiError string
}

// set: the fake agent writes a field it has not learned yet as null.
func set(m json.RawMessage) bool { return len(m) > 0 && string(m) != "null" }

type mcpServer struct {
	Type, Name, URL string
	Headers         []struct{ Name, Value string }
}

type claimed struct {
	RunID, RunToken, Brief, McpURL string
}

// f09 is F08's world (a built daemon, a pod, two observers) with the fake agent as the fake harness, behind a recording proxy.
type f09 struct {
	*f08
	px    *recorder
	agent string
	run   claimed
}

func newF09(t *testing.T) *f09 {
	f := &f09{f08: newF08(t)}
	root, _ := filepath.Abs("..")
	f.agent = filepath.Join(t.TempDir(), "fakeagent")
	cmd := exec.Command("go", "build", "-o", f.agent, "./e2e/testdata/fakeagent")
	cmd.Dir = root
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("go build ./e2e/testdata/fakeagent: %v\n%s", err, out)
	}
	f.px = newRecorder(t, f.srv.base())
	f.env[1] = "JELLYMOON_SERVER=" + f.px.URL // the daemon talks to the server through the proxy
	return f
}

// harness points the fake harness at a record file; args are the fake agent's own (not tokens).
func (f *f09) harness(record string, args ...string) {
	f.env = append(f.env, "JELLYMOON_HARNESS_FAKE="+strings.Join(append([]string{f.agent, record}, args...), " "))
}

// assign creates a task for the agent, which wakes it.
func (f *f09) assign(title string) {
	f.obs[0].send("task.create", map[string]any{"title": title, "kind": "chore", "assignee": f.agentID})
}

// recorded waits for the fake agent's record to satisfy ok.
func (f *f09) recorded(path string, ok func(agentRec) bool) (agentRec, error) {
	var r agentRec
	for end := time.Now().Add(15 * time.Second); time.Now().Before(end); time.Sleep(25 * time.Millisecond) {
		if raw, err := os.ReadFile(path); err == nil && json.Unmarshal(raw, &r) == nil && ok(r) {
			return r, nil
		}
	}
	return r, fmt.Errorf("the fake agent's record %s never satisfied the wait within 15 s: %+v\ndaemon output:\n%s", path, r, f.daemon.text())
}

func (f *f09) servers(r agentRec) []mcpServer {
	var p struct{ McpServers []mcpServer }
	_ = json.Unmarshal(r.SessionNew, &p)
	return p.McpServers
}

var runPath = regexp.MustCompile(`^/v1/runs$`)

// Step 1: a task assigned to the agent wakes it, and the daemon claims the wake with its computer token.
func (f *f09) claims(record string) error {
	if err := f.registersComputer(); err != nil {
		return err
	}
	if err := f.addsAgent(); err != nil {
		return err
	}
	if err := f.all("online", []int{0, 0}, 5*time.Second); err != nil {
		return err
	}
	f.assign("wake the agent")
	h, ok := f.px.find("POST", runPath, 10*time.Second)
	if !ok {
		return fmt.Errorf("the daemon never called POST /v1/runs after the task was assigned\ndaemon output:\n%s", f.daemon.text())
	}
	var body struct{ WakeID string }
	_ = json.Unmarshal(h.In, &body)
	if !regexp.MustCompile(`^wak_[0-9A-Z]{26}$`).MatchString(body.WakeID) || h.Auth != "Bearer "+f.comp.Token || h.Status != 200 {
		return fmt.Errorf("POST /v1/runs: HTTP %d, wakeId %q, computer token sent=%v; want 200, a wak_ id and the jmc_", h.Status, body.WakeID, h.Auth == "Bearer "+f.comp.Token)
	}
	_ = json.Unmarshal(h.Out, &f.run)
	return nil
}

// Step 2.
func (f *f09) response() error {
	r := f.run
	if !regexp.MustCompile(`^run_[0-9A-Z]{26}$`).MatchString(r.RunID) || !regexp.MustCompile(`^jmr_[A-Za-z0-9]{43}$`).MatchString(r.RunToken) || r.Brief == "" || r.McpURL != f.px.URL+"/mcp" {
		return fmt.Errorf("the claim response carries runId=%q jmr_ shaped=%v brief=%d chars mcpUrl=%q, want run_, a 47-character jmr_, a brief and %s/mcp",
			r.RunID, strings.HasPrefix(r.RunToken, "jmr_"), len(r.Brief), r.McpURL, f.px.URL)
	}
	return nil
}

// Step 3: session/new carries the MCP server with the run token, and the brief is the first prompt.
func (f *f09) sessionNew(record string) error {
	rec, err := f.recorded(record, func(r agentRec) bool { return set(r.Prompt) })
	if err != nil {
		return err
	}
	srv := f.servers(rec)
	if len(srv) != 1 || srv[0].Type != "http" || srv[0].URL != f.run.McpURL || len(srv[0].Headers) != 1 || srv[0].Headers[0].Name != "Authorization" || srv[0].Headers[0].Value != "Bearer "+f.run.RunToken {
		return fmt.Errorf("session/new mcpServers = %+v, want one http server at %s with Authorization: Bearer <the run's jmr_>", srv, f.run.McpURL)
	}
	var np struct{ Cwd string }
	var pr struct{ Prompt []struct{ Type, Text string } }
	_ = json.Unmarshal(rec.SessionNew, &np)
	_ = json.Unmarshal(rec.Prompt, &pr)
	if !filepath.IsAbs(np.Cwd) || len(pr.Prompt) != 1 || pr.Prompt[0].Text != f.run.Brief {
		return fmt.Errorf("session/new cwd %q (want absolute); first prompt %+v (want the brief)", np.Cwd, pr.Prompt)
	}
	return nil
}

// Step 4: the agent called jm_whoami over MCP and the pod answered with the agent, the pod and the run.
func (f *f09) whoami(record string) error {
	rec, err := f.recorded(record, func(r agentRec) bool { return set(r.Whoami) || r.WhoamiError != "" })
	if err != nil {
		return err
	}
	var out struct {
		Result struct {
			IsError           bool
			StructuredContent struct {
				Agent Actor
				PodID string
				RunID string
			}
		}
	}
	_ = json.Unmarshal(rec.Whoami, &out)
	got := out.Result.StructuredContent
	if got.Agent.ID != f.agentID || got.PodID != f.pod || got.RunID != f.run.RunID || out.Result.IsError {
		return fmt.Errorf("jm_whoami answered agent %q pod %q run %q (error=%v %s), want %s %s %s\nraw: %s", got.Agent.ID, got.PodID, got.RunID, out.Result.IsError, rec.WhoamiError, f.agentID, f.pod, f.run.RunID, rec.Whoami)
	}
	return nil
}

// Step 5: the agent's argv and env hold no token. The run token reached it in its MCP server config, and no computer or user token did.
func (f *f09) tokens(record string) error {
	rec, _ := f.recorded(record, func(r agentRec) bool { return set(r.Prompt) })
	where := map[string]string{"argv": strings.Join(rec.Argv, "\n"), "env": strings.Join(rec.Env, "\n"), "session/new": string(rec.SessionNew), "initialize": string(rec.Initialize), "prompt": string(rec.Prompt)}
	for name, text := range where {
		for _, secret := range []string{f.comp.Token, f.owner.tok} {
			if strings.Contains(text, secret) {
				return fmt.Errorf("the agent's %s holds the computer or user token", name)
			}
		}
		if name != "session/new" && secretShape.MatchString(text) {
			return fmt.Errorf("the agent's %s holds a token-shaped string %s", name, secretShape.FindString(text))
		}
	}
	if !strings.Contains(where["session/new"], f.run.RunToken) {
		return fmt.Errorf("the run token is not in the agent's session/new")
	}
	return nil
}

// Step 6: when the agent finishes, the daemon ends the run as completed.
func (f *f09) ends() error {
	h, ok := f.px.find("POST", regexp.MustCompile(`^/v1/runs/`+f.run.RunID+`/end$`), 10*time.Second)
	if !ok || h.Status != 200 || !bytes.Contains(h.In, []byte(`"completed"`)) || h.Auth != "Bearer "+f.comp.Token {
		return fmt.Errorf("POST /v1/runs/%s/end: found=%v HTTP %d body %s, want 200 completed with the computer token", f.run.RunID, ok, h.Status, h.In)
	}
	return nil
}

// stops: a daemon that is stopped takes an agent that will not leave on its own with it, and ends its run as failed.
func (f *f09) stops(record string) error {
	f.daemon.signal(syscall.SIGTERM)
	if _, ok := f.daemon.finish(15 * time.Second); !ok {
		return fmt.Errorf("the daemon did not exit within 15 s of SIGTERM")
	}
	f.harness(record, "hold")
	marks := f.marks()
	f.startDaemon("daemon2")
	if err := f.all("online", marks, 5*time.Second); err != nil {
		return err
	}
	f.assign("hold the agent")
	rec, err := f.recorded(record, func(r agentRec) bool { return set(r.Whoami) })
	if err != nil {
		return err
	}
	f.daemon.signal(syscall.SIGTERM)
	if _, ok := f.daemon.finish(15 * time.Second); !ok {
		return fmt.Errorf("the daemon did not exit within 15 s of SIGTERM with an agent running")
	}
	for end := time.Now().Add(5 * time.Second); syscall.Kill(rec.PID, 0) == nil; time.Sleep(25 * time.Millisecond) {
		if time.Now().After(end) {
			_ = syscall.Kill(-rec.PID, syscall.SIGKILL)
			return fmt.Errorf("the agent (pid %d) still runs 5 s after the daemon stopped", rec.PID)
		}
	}
	if _, ok := f.px.find("POST", regexp.MustCompile(`^/v1/runs/run_[0-9A-Z]{26}/end$`), time.Second); !ok {
		return fmt.Errorf("no run was ended")
	}
	return nil
}

// leaksNothing: no token reaches the daemon's output.
func (f *f09) leaksNothing() error {
	if hits := leaks(f.procs, "", f.owner.tok, f.comp.Token, f.run.RunToken); len(hits) > 0 {
		return fmt.Errorf("%s", strings.Join(hits, "; "))
	}
	return nil
}

// TestF09: on run.wake the daemon claims a run and starts the agent with a run token and the pod's MCP server.
func TestF09(t *testing.T) {
	needFake(t)
	track(t)
	f := newF09(t)
	record, held := filepath.Join(t.TempDir(), "agent.json"), filepath.Join(t.TempDir(), "held.json")
	f.harness(record)
	p := &stepper{out: os.Stdout, events: func() []Frame { return f.obs[0].snapshot(0) }}
	defer p.count(t)
	p.step("the daemon receives run.wake and calls POST /v1/runs", func() error { return f.claims(record) })
	p.step("the response carries runId, a jmr_ token, a brief and the MCP URL", f.response)
	p.step("the fake ACP agent starts with the MCP server passed in session/new", func() error { return f.sessionNew(record) })
	p.step("jm_whoami over MCP returns the agent, pod and run", func() error { return f.whoami(record) })
	p.step("the agent's argv and env never hold jmc_ or jmu_, and the jmr_ reaches it only in its MCP server config", func() error { return f.tokens(record) })
	p.step("when the agent finishes the daemon ends the run as completed", f.ends)
	p.step("a stopped daemon takes an agent that will not leave with it", func() error { return f.stops(held) })
	p.step("no token reaches the daemon's output", f.leaksNothing)
	if p.ok != p.run {
		t.Fatalf("%d of %d steps passed", p.ok, p.run)
	}
}
