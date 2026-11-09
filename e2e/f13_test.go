package e2e

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"syscall"
	"testing"
	"time"
)

// TestF13: tokens are scoped. A run cannot cross pods, an ended run is dead, an `actor` in MCP tool arguments
// changes nothing, the production builds have no /__test handler, and a pod owner's stop ends a live run at once.
func TestF13(t *testing.T) {
	needFake(t)
	par(t)
	w := newWorld(t)
	keep := w.f13CrossPod()
	w.f13EndedRun(keep)
	w.f13ActorIgnored(keep)
	w.f13Production()
	w.f13Stop()
}

// mcpReply is one tools/call answer: a transport status, whether the tool refused, and the pod's error code or output.
type mcpReply struct {
	Status  int
	Refused bool
	Code    string
	Out     map[string]any
}

// mcp sends one tools/call with raw JSON arguments (a string, so a key like __proto__ stays as typed).
func (w *world) mcp(tok, tool, args string) mcpReply {
	w.t.Helper()
	body := `{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"` + tool + `","arguments":` + args + `}}`
	req, _ := http.NewRequest("POST", w.srv.base()+"/mcp", strings.NewReader(body))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+tok)
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		w.t.Fatalf("POST /mcp: %v", err)
	}
	defer resp.Body.Close()
	raw, _ := io.ReadAll(resp.Body)
	r := mcpReply{Status: resp.StatusCode}
	var env struct {
		Result struct {
			IsError           bool
			StructuredContent map[string]any
		}
		Error *struct{ Message string }
	}
	if json.Unmarshal(raw, &env) == nil {
		r.Refused, r.Out = env.Result.IsError || env.Error != nil, env.Result.StructuredContent
		r.Code, _ = r.Out["code"].(string)
		if env.Error != nil {
			r.Code = "jsonrpc"
		}
	} else if resp.StatusCode == 401 {
		_ = json.Unmarshal(raw, &r.Out)
		r.Code, _ = r.Out["code"].(string)
	}
	return r
}

// rpc sends a JSON-RPC method that is not a tool call (initialize, tools/list, ping) and returns the HTTP status.
func (w *world) rpc(tok, method string) int {
	w.t.Helper()
	req, _ := http.NewRequest("POST", w.srv.base()+"/mcp", strings.NewReader(`{"jsonrpc":"2.0","id":1,"method":"`+method+`","params":{}}`))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+tok)
	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		w.t.Fatalf("POST /mcp %s: %v", method, err)
	}
	resp.Body.Close()
	return resp.StatusCode
}

// wantDeadRPC: a dead run token gets 401 on every JSON-RPC method, not only on a tool call.
func (w *world) wantDeadRPC(tok, what string) {
	w.t.Helper()
	for _, m := range []string{"initialize", "tools/list", "ping"} {
		if st := w.rpc(tok, m); st != 401 {
			w.t.Errorf("%s, MCP %s: HTTP %d, want 401", what, m, st)
		}
	}
}

// wantDead: a dead run token over /mcp gets unauthorized, either as the transport's 401 or as the tool's refusal carrying
// the HTTP route's own error (the adapter forwards the route's answer).
func (w *world) wantDead(r mcpReply, what string) {
	w.t.Helper()
	if r.Code != "unauthorized" || (r.Status != 401 && !(r.Status == 200 && r.Refused)) {
		w.t.Errorf("%s: HTTP %d refused=%v code %q, want unauthorized", what, r.Status, r.Refused, r.Code)
	}
}

type f13Msg struct {
	Summary string
	Author  Actor
}

func (w *world) f13Feed() []f13Msg {
	var out struct{ Messages []f13Msg }
	w.must(w.c.do("GET", w.podPath("feed")+"?limit=50", w.owner.tok, nil, &out), "read the feed")
	return out.Messages
}

// Step 1: a run token from pod A on pod B is forbidden, on every route and over MCP, and the pod it names in an argument is not honoured.
func (w *world) f13CrossPod() *run {
	r := w.startRun(w.a1, "own pod")
	var b struct {
		ID string `json:"id"`
	}
	w.must(w.c.do("POST", "/v1/pods", w.owner.tok, map[string]any{"name": "pod-b"}, &b), "create pod B")
	for _, c := range []struct{ method, path string }{
		{"GET", "/v1/pods/" + b.ID + "/me"}, {"GET", "/v1/pods/" + b.ID + "/tasks"}, {"GET", "/v1/pods/" + b.ID + "/feed"},
		{"POST", "/v1/pods/" + b.ID + "/messages"}, {"POST", "/v1/pods/" + b.ID + "/invites"}, {"POST", "/v1/pods/" + b.ID + "/runs/" + r.id + "/stop"},
		{"GET", "/v1/pods/__proto__/me"}, {"POST", "/v1/pods/constructor/messages"}, {"GET", "/v1/pods/pod_00000000000000000000000000/tasks"},
	} {
		var in any
		if c.method == "POST" {
			in = post(nil)
		}
		w.wantErr(w.runCall(r, c.method, c.path, in, nil), 403, "forbidden", "pod A's run token: "+c.method+" "+c.path)
	}
	// A user token on a pod that is not there, named like an object prototype key, is a plain not_found.
	for _, id := range []string{"__proto__", "constructor", "toString"} {
		w.wantErr(w.c.do("GET", "/v1/pods/"+id+"/me", w.owner.tok, nil, nil), 404, "not_found", "user token, pod "+id)
	}
	// Over MCP the pod is the token's. A podId argument naming pod B lands on pod A, and a post there is stored in pod A.
	me := w.mcp(r.tok, "jm_whoami", `{"podId":"`+b.ID+`"}`)
	if me.Status != 200 || me.Refused || me.Out["podId"] != w.pod {
		w.t.Errorf("jm_whoami naming pod B: HTTP %d refused=%v %+v, want pod %s", me.Status, me.Refused, me.Out, w.pod)
	}
	w.mcp(r.tok, "jm_post", `{"summary":"meant for B","kind":"fyi","status":"done","podId":"`+b.ID+`"}`)
	var bFeed struct{ Messages []f13Msg }
	w.must(w.c.do("GET", "/v1/pods/"+b.ID+"/feed?limit=50", w.owner.tok, nil, &bFeed), "read pod B's feed")
	if len(bFeed.Messages) != 0 {
		w.t.Errorf("pod B holds %+v after a post from pod A's run, want nothing", bFeed.Messages)
	}
	w.must(w.runCall(r, "GET", w.podPath("me"), nil, nil), "the same token on its own pod")
	return r
}

// Step 2: after POST /v1/runs/:run/end the same jmr_ is unauthorized, over HTTP and MCP; only the hosting computer may end a run.
func (w *world) f13EndedRun(keep *run) {
	r := w.startRun(w.a2, "to be ended")
	w.wantErr(w.c.do("POST", "/v1/runs/"+r.id+"/end", w.compB.tok, map[string]any{"outcome": "completed"}, nil), 403, "forbidden", "another computer ends the run")
	w.wantErr(w.c.do("POST", "/v1/runs/"+r.id+"/end", w.owner.tok, map[string]any{"outcome": "completed"}, nil), 403, "forbidden", "a user token ends the run")
	w.wantErr(w.c.do("POST", "/v1/runs/"+r.id+"/end", r.tok, map[string]any{"outcome": "completed"}, nil), 403, "forbidden", "the run's own token ends the run")
	w.must(w.runCall(r, "GET", w.podPath("me"), nil, nil), "the run token while the run is live")
	for _, id := range []string{"__proto__", "constructor"} {
		if rs := w.c.do("POST", "/v1/runs/"+id+"/end", w.compA.tok, map[string]any{"outcome": "completed"}, nil); rs.Status != 404 && rs.Status != 400 {
			w.t.Errorf("end of run %q: HTTP %d %s, want 404 or 400", id, rs.Status, rs.Err.Code)
		}
	}
	w.must(w.c.do("POST", "/v1/runs/"+r.id+"/end", w.compA.tok, map[string]any{"outcome": "completed"}, nil), "the hosting computer ends the run")
	r.ended = true
	w.wantErr(w.c.do("GET", w.podPath("me"), r.tok, nil, nil), 401, "unauthorized", "ended run, GET me")
	w.wantErr(w.c.do("POST", w.podPath("messages"), r.tok, post(nil), nil), 401, "unauthorized", "ended run, post")
	w.wantErr(w.c.do("POST", w.podPath("tasks", "tsk_00000000000000000000000000", "claim"), r.tok, map[string]any{}, nil), 401, "unauthorized", "ended run, claim")
	w.wantDead(w.mcp(r.tok, "jm_whoami", `{}`), "ended run, MCP")
	w.wantDead(w.mcp(r.tok, "jm_post", `{"summary":"after the end","kind":"fyi","status":"done"}`), "ended run, MCP post")
	w.wantDeadRPC(r.tok, "ended run")
	for _, m := range w.f13Feed() {
		if m.Summary == "after the end" {
			w.t.Errorf("a post from an ended run is stored: %+v", m)
		}
	}
	// The token of a different, still live run is untouched.
	w.must(w.runCall(keep, "GET", w.podPath("me"), nil, nil), "another run's token after this run ended")
}

// Step 3: the actor of an MCP call is the token's run. A call carrying `actor` is refused 400 and takes no effect (a body
// field the core has no place for is refused, never dropped); a read tool never reads it. Nothing is recorded under the named actor.
func (w *world) f13ActorIgnored(r *run) {
	owner := fmt.Sprintf(`{"kind":"human","id":"%s","displayName":"olivia"}`, w.owner.actor.ID)
	for _, args := range []string{
		`{"summary":"as olivia","kind":"fyi","status":"done","actor":` + owner + `}`,
		`{"summary":"as olivia","kind":"fyi","status":"done","actor":"` + w.owner.actor.ID + `"}`,
		`{"summary":"as olivia","kind":"fyi","status":"done","author":` + owner + `}`,
		`{"summary":"as olivia","kind":"fyi","status":"done","__proto__":{"actor":` + owner + `}}`,
		`{"summary":"as olivia","kind":"fyi","status":"done","constructor":{"actor":` + owner + `}}`,
	} {
		rep := w.mcp(r.tok, "jm_post", args)
		if rep.Status != 200 || !rep.Refused || rep.Code != "invalid_input" {
			w.t.Errorf("jm_post %.60s...: HTTP %d refused=%v code %q, want refused invalid_input", args[40:], rep.Status, rep.Refused, rep.Code)
		}
	}
	for _, m := range w.f13Feed() {
		if m.Summary == "as olivia" {
			w.t.Errorf("a refused post took effect: %+v", m)
		}
	}
	// A valid post that carries nothing extra is attributed to the run's agent, whatever else the request says.
	ok := w.mcp(r.tok, "jm_post", `{"summary":"as the run","kind":"fyi","status":"done"}`)
	if ok.Refused || ok.Status != 200 {
		w.t.Fatalf("a plain jm_post: HTTP %d refused=%v %+v", ok.Status, ok.Refused, ok.Out)
	}
	found := false
	for _, m := range w.f13Feed() {
		if m.Summary == "as the run" {
			found = true
			if m.Author.ID != w.a1.id || m.Author.Kind != "agent" {
				w.t.Errorf("the post is by %+v, want agent %s", m.Author, w.a1.id)
			}
		}
	}
	if !found {
		w.t.Errorf("the plain post is not in the feed")
	}
	// The read tools take no actor: it is not read, and the answer is the run's own.
	me := w.mcp(r.tok, "jm_whoami", `{"actor":`+owner+`}`)
	agent, _ := me.Out["agent"].(map[string]any)
	if me.Refused || agent["id"] != w.a1.id || me.Out["runId"] != r.id {
		w.t.Errorf("jm_whoami with an actor: refused=%v %+v, want the run %s and agent %s", me.Refused, me.Out, r.id, w.a1.id)
	}
	// A tool a run may not use is no more open to a run that names an actor: run_stop is user-only.
	stop := w.mcp(r.tok, "jm_run_stop", `{"runId":"`+r.id+`","actor":`+owner+`}`)
	if !stop.Refused {
		w.t.Errorf("jm_run_stop by a run naming the owner as actor was not refused: %+v", stop.Out)
	}
	w.must(w.runCall(r, "GET", w.podPath("me"), nil, nil), "the run token after the stop attempt")
}

// Step 4: the production builds carry no /__test handler. Each is run, not read: the server over real HTTP without ENV=local
// (server/test/production.test.ts, also part of `make test`), and the Worker's production entry under wrangler.
func (w *world) f13Production() {
	root, _ := filepath.Abs("..")
	cmd := exec.Command("bun", "test", "./server/test/production.test.ts")
	cmd.Dir = root
	if out, err := cmd.CombinedOutput(); err != nil {
		w.t.Errorf("the server's production check failed: %v\n%s", err, out)
	}
	base, hc := w.f13Worker(root), &http.Client{Timeout: 2 * time.Second}
	for _, path := range []string{"/__test", "/__test/", "/__test/session", "/__test/clock", "/__test/evict/pod_1", "/__test/discard/pod_1", "/__test/instance/pod_1", "/__test/woke/pod_1",
		"/__test/stored-alarm/pod_1", "//__test/session", "/%5F%5Ftest/session", "/_in/evict", "/_in/discard"} {
		for _, method := range []string{"GET", "POST"} {
			req, _ := http.NewRequest(method, base+path, strings.NewReader(`{"name":"a","now":1}`))
			req.Header.Set("Authorization", "Bearer sekret")
			resp, err := hc.Do(req)
			if err != nil {
				w.t.Errorf("%s %s on the production Worker: %v", method, path, err)
				continue
			}
			resp.Body.Close()
			if resp.StatusCode != 404 {
				w.t.Errorf("%s %s on the production Worker: HTTP %d, want 404", method, path, resp.StatusCode)
			}
		}
	}
}

// f13Worker starts the Worker's production entry under wrangler and returns its base URL once /healthz answers 200. A start
// that fails the way startServer retries is tried once more on fresh ports; a failure names its cause from wrangler's output.
func (w *world) f13Worker(root string) string {
	for try := 1; ; try++ {
		port := freePort(w.t)
		out, why := w.f13WorkerStart(root, port)
		if why == "" {
			return fmt.Sprintf("http://127.0.0.1:%d", port)
		}
		line, retry := startRetry(w.t.Name(), port, try+1, out.bytes())
		if try == 2 || !retry {
			w.t.Fatalf("the production Worker %s; log tail:\n%s", why, out.tail(20))
		}
		fmt.Println(line)
	}
}

// f13WorkerStart runs wrangler once and waits for /healthz. It returns wrangler's output and, on failure, what went wrong:
// the process exiting ends the wait at once, so a crash at second 2 is not reported as a 60 s timeout.
func (w *world) f13WorkerStart(root string, port int) (*logBuf, string) {
	dir, out := w.t.TempDir(), &logBuf{}
	wr := exec.Command("bun", filepath.Join(root, "node_modules/wrangler/bin/wrangler.js"), "dev", "-c", "wrangler.prod.toml", "--local", "--ip", "127.0.0.1", "--port", fmt.Sprint(port),
		"--inspector-port", fmt.Sprint(freePort(w.t, port)), "--persist-to", filepath.Join(dir, "state"), "--show-interactive-dev-session=false")
	wr.Dir = filepath.Join(root, "worker")
	wr.Env = append(os.Environ(), "WRANGLER_SEND_METRICS=false", "NO_UPDATE_NOTIFIER=1", "CI=1", "XDG_CONFIG_HOME="+filepath.Join(dir, "cfg"), "WRANGLER_LOG_PATH="+filepath.Join(dir, "logs"),
		"CLOUDFLARE_API_TOKEN=", "CLOUDFLARE_ACCOUNT_ID=", "CLOUDFLARE_API_KEY=", "CLOUDFLARE_EMAIL=")
	wr.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	wr.Stdout, wr.Stderr, wr.WaitDelay = out, out, 2*time.Second // a child still holding the pipe does not hold Wait
	if err := wr.Start(); err != nil {
		w.t.Fatalf("cannot start the production Worker: %v", err)
	}
	exited := make(chan struct{})
	go func() { _ = wr.Wait(); close(exited) }()
	var once sync.Once
	up := false // a start that never came up has its tail in the Fatalf already
	stop := func() { once.Do(func() { _ = syscall.Kill(-wr.Process.Pid, syscall.SIGKILL); <-exited }) }
	w.t.Cleanup(func() {
		stop()
		if up && w.t.Failed() {
			w.t.Logf("production Worker log tail:\n%s", out.tail(40))
		}
	})
	hc := &http.Client{Timeout: 2 * time.Second}
	for end := time.Now().Add(60 * time.Second); time.Now().Before(end); time.Sleep(100 * time.Millisecond) {
		select {
		case <-exited:
			stop()
			return out, "exited before /healthz answered 200"
		default:
		}
		if r, err := hc.Get(fmt.Sprintf("http://127.0.0.1:%d/healthz", port)); err == nil {
			r.Body.Close()
			if up = r.StatusCode == 200; up {
				return out, ""
			}
		}
	}
	stop()
	return out, "did not answer /healthz within 60 s"
}

// Step 5: a pod owner's stop of a live run: at once the token is dead, the run is cancelled, the claim is released with cause
// stopped and `by`, and the hosting computer receives run.stop. It works with the computer offline, and only a person may do it.
func (w *world) f13Stop() {
	r := w.startRun(w.a1, "to be stopped")
	tk := w.newTask("held by the stopped run", "")
	w.mustClaim(r, tk.ID)
	// Not a person with the right: a member who neither owns the agent nor the pod, and the run itself.
	w.wantErr(w.c.do("POST", w.podPath("runs", r.id, "stop"), w.cara.tok, map[string]any{}, nil), 403, "forbidden", "a plain member stops a run")
	w.wantErr(w.runCall(r, "POST", w.podPath("runs", r.id, "stop"), map[string]any{}, nil), 403, "forbidden", "a run token stops a run")
	w.must(w.runCall(r, "GET", w.podPath("me"), nil, nil), "the run token after refused stops")
	for _, id := range []string{"__proto__", "constructor"} {
		if rs := w.c.do("POST", w.podPath("runs", id, "stop"), w.owner.tok, map[string]any{}, nil); rs.Status != 404 && rs.Status != 400 {
			w.t.Errorf("stop of run %q: HTTP %d %s, want 404 or 400", id, rs.Status, rs.Err.Code)
		}
	}
	cm, om := w.compA.sock.mark(), w.sO.mark()
	var out struct{ RunID, Outcome string }
	w.must(w.c.do("POST", w.podPath("runs", r.id, "stop"), w.owner.tok, map[string]any{"reason": "owner says stop"}, &out), "the pod owner stops the live run")
	if out.RunID != r.id || out.Outcome != "cancelled" {
		w.t.Errorf("stop answered %+v, want run %s cancelled", out, r.id)
	}
	// At once: the very next call, with no wait for the daemon, is unauthorized.
	w.wantErr(w.c.do("GET", w.podPath("me"), r.tok, nil, nil), 401, "unauthorized", "the stopped run's token, next call")
	w.wantDead(w.mcp(r.tok, "jm_whoami", `{}`), "the stopped run's token over MCP")
	w.wantDeadRPC(r.tok, "stopped run")
	var stop Released
	w.expect(w.compA.sock, cm, func(f Frame) bool { f.dec(&stop); return f.Type == "run.stop" && stop.RunID == r.id }, "run.stop on the hosting computer")
	if stop.Reason != "owner says stop" {
		w.t.Errorf("run.stop carries %+v, want the reason", stop)
	}
	var rl Released
	w.expect(w.sO, om, released(tk.ID, "stopped"), "claim.released stopped").dec(&rl)
	if rl.RunID != r.id || rl.By == nil || rl.By.ID != w.owner.actor.ID {
		w.t.Errorf("claim.released %+v, want run %s by the owner", rl, r.id)
	}
	w.must(w.c.do("POST", w.podPath("runs", r.id, "stop"), w.owner.tok, map[string]any{}, &out), "a second stop")
	if out.Outcome != "cancelled" {
		w.t.Errorf("a second stop answered %+v, want the outcome unchanged (cancelled)", out)
	}
	// Offline: the daemon's socket is gone, and the stop still revokes the token before anyone listens.
	r2 := w.startRun(w.a2, "stopped offline")
	w.compA.sock.close()
	w.eventually("the computer is offline", func() (bool, string) { return w.compA.sock.isDead(), "the socket is still open" })
	w.must(w.c.do("POST", w.podPath("runs", r2.id, "stop"), w.ben.tok, map[string]any{}, &out), "the agent's owner stops a run on an offline computer")
	if out.Outcome != "cancelled" {
		w.t.Errorf("offline stop answered %+v, want cancelled", out)
	}
	w.wantErr(w.c.do("GET", w.podPath("me"), r2.tok, nil, nil), 401, "unauthorized", "the token of a run stopped while its computer was offline")
	w.f13Reconnect(r2)
}

// f13Reconnect: the computer comes back and finds run.stop was not lost to the pod's state; its next end of the run is not an error that revives the token.
func (w *world) f13Reconnect(r *run) {
	w.compA.sock = w.open(w.compA.tok, "computer-a-again")
	w.wantErr(w.c.do("GET", w.podPath("me"), r.tok, nil, nil), 401, "unauthorized", "the token after the computer reconnected")
}
