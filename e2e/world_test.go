package e2e

import (
	"fmt"
	"strings"
	"sync"
	"testing"
	"time"
)

type user struct {
	tok   string
	actor Actor
}

type computer struct {
	id, tok string
	sock    *sock
}
type agent struct {
	id    string
	actor Actor
	comp  *computer
}
type run struct {
	id, tok string
	agent   *agent
	ended   bool
}

// world is one fresh server with an owner, two members, two computers, three agents and live sockets.
// ben owns computer A (agents a1, a2) and cara owns computer B (agent b1); olivia owns the pod.
type world struct {
	t            *testing.T
	srv          *server
	c            *client
	pod          string
	clock        int64
	owner        user
	ben, cara    user
	compA, compB *computer
	a1, a2, b1   *agent
	sO, sBen     *sock
	sCara        *sock
	mu           sync.Mutex
	renewed      map[string]int64
	runs         []*run
	contested    string
}

func wantID(t *testing.T, prefix, s string) {
	t.Helper()
	n := 26 // ULID
	if prefix[:2] == "jm" {
		n = 43 // token
	}
	if !strings.HasPrefix(s, prefix) || len(s) != len(prefix)+n {
		t.Fatalf("id %q is not %s plus %d characters", s, prefix, n)
	}
}

func newWorld(t *testing.T) *world { return newWorldAt(t, fakeBase) }

// newWorldAt is newWorld with the clock set to `clock` (unix ms) before anything else; real-clock mode ignores it.
func newWorldAt(t *testing.T, clock int64) *world {
	t.Helper()
	need(t)
	track(t)
	w := &world{t: t, renewed: map[string]int64{}}
	w.srv = startServer(t)
	w.c = newClient(t, w.srv.base())
	if !realClock {
		w.setClock(clock)
	}
	w.owner, w.ben, w.cara = w.session("olivia"), w.session("ben"), w.session("cara")
	var pod struct {
		ID string `json:"id"`
	}
	w.must(w.c.do("POST", "/v1/pods", w.owner.tok, map[string]any{"name": "contract"}, &pod), "create pod")
	w.pod = pod.ID
	var inv struct {
		Code string `json:"code"`
	}
	w.must(w.c.do("POST", "/v1/pods/"+w.pod+"/invites", w.owner.tok, map[string]any{"maxUses": 2}, &inv), "create invite")
	for _, u := range []user{w.ben, w.cara} {
		w.must(w.c.do("POST", "/v1/invites/"+inv.Code+"/accept", u.tok, nil, nil), "accept invite")
	}
	w.compA, w.compB = w.register(w.ben, "mac-a"), w.register(w.cara, "mac-b")
	w.a1, w.a2 = w.addAgent(w.compA, "agent-one"), w.addAgent(w.compA, "agent-two")
	w.b1 = w.addAgent(w.compB, "agent-bee")
	w.sO, w.sBen, w.sCara = w.open(w.owner.tok, "olivia"), w.open(w.ben.tok, "ben"), w.open(w.cara.tok, "cara")
	w.compA.sock, w.compB.sock = w.open(w.compA.tok, "computer-a"), w.open(w.compB.tok, "computer-b")
	return w
}

func (w *world) must(r res, what string) {
	w.t.Helper()
	if !r.ok() {
		w.t.Fatalf("%s: HTTP %d code=%q message=%q", what, r.Status, r.Err.Code, r.Err.Message)
	}
}

func (w *world) session(name string) user {
	u, err := seedUser(w.c, name)
	if err != nil {
		w.t.Fatalf("POST /__test/session: %v (the test-route contract is not implemented; is ENV=local?)", err)
	}
	return u
}

func (w *world) register(u user, name string) *computer {
	var out struct {
		ComputerID string `json:"computerId"`
		Token      string `json:"token"`
	}
	w.must(w.c.do("POST", "/v1/computers", u.tok, map[string]any{"name": name, "os": "darwin", "arch": "arm64"}, &out), "register computer")
	wantID(w.t, "jmc_", out.Token)
	return &computer{id: out.ComputerID, tok: out.Token}
}

func (w *world) addAgent(c *computer, name string) *agent {
	var out struct {
		AgentID string `json:"agentId"`
	}
	w.must(w.c.do("POST", "/v1/pods/"+w.pod+"/agents", c.tok, map[string]any{"harness": "fake", "displayName": name}, &out), "register agent")
	return &agent{id: out.AgentID, actor: Actor{Kind: "agent", ID: out.AgentID, DisplayName: name}, comp: c}
}

func (w *world) dial(tok, name string) *sock {
	conn, br, err := wsDial(fmt.Sprintf("127.0.0.1:%d", w.srv.port), "/v1/pods/"+w.pod+"/ws", tok)
	if err != nil {
		w.t.Fatalf("%s: WebSocket /v1/pods/:pod/ws: %v", name, err)
	}
	s := newSock(name, w.pod, conn, br)
	w.t.Cleanup(s.close)
	return s
}

// open connects, waits for hello and subscribes from the pod's head: live events only.
func (w *world) open(tok, name string) *sock {
	s := w.dial(tok, name)
	h, ok := s.waitAfter(0, isType("hello"), 5*time.Second)
	if !ok {
		w.t.Fatalf("%s: no hello within 5 s of the upgrade", name)
	}
	var hello struct {
		HeadSeq int64 `json:"headSeq"`
	}
	h.dec(&hello)
	s.send("subscribe", map[string]any{"sinceSeq": hello.HeadSeq})
	return s
}

func (w *world) openSince(tok, name string, since int64) *sock {
	s := w.dial(tok, name)
	s.send("subscribe", map[string]any{"sinceSeq": since})
	return s
}

func (w *world) setClock(now int64) {
	w.t.Helper()
	if realClock {
		w.t.Fatalf("suite bug: /__test/clock called in real-clock mode")
	}
	r := w.c.do("POST", "/__test/clock", "", map[string]any{"now": now}, nil)
	if !r.ok() {
		w.t.Fatalf("POST /__test/clock: HTTP %d (test-route contract not implemented; is ENV=local?)", r.Status)
	}
	w.clock = now
}

func (w *world) advance(d int64) { w.setClock(w.clock + d) }

// now is the time the suite stamps a renewal with: the clock it set, or the wall clock in real mode.
func (w *world) now() int64 {
	if realClock {
		return time.Now().UnixMilli()
	}
	return w.clock
}

// stamp records a renewal the suite caused. Every expiry and lateness check reads this ledger.
func (w *world) stamp(runID string, at int64) {
	w.mu.Lock()
	defer w.mu.Unlock()
	if at > w.renewed[runID] {
		w.renewed[runID] = at
	}
}

// expected is the run's last recorded renewal plus 60 000 ms: the earliest its claims may expire.
func (w *world) expected(r *run) int64 {
	w.mu.Lock()
	defer w.mu.Unlock()
	return w.renewed[r.id] + leaseMs
}

// runCall makes a request on a run token and records a renewal when it succeeds.
func (w *world) runCall(r *run, method, path string, in, out any) res {
	at := w.now()
	rs := w.c.do(method, path, r.tok, in, out)
	if rs.ok() {
		w.stamp(r.id, at)
	}
	return rs
}

// heartbeat sends presence.heartbeat from a computer and records a renewal for each live run it hosts.
func (w *world) heartbeat(c *computer) {
	at := w.now()
	c.sock.send("presence.heartbeat", map[string]any{"computerId": c.id})
	w.mu.Lock()
	live := append([]*run(nil), w.runs...)
	w.mu.Unlock()
	for _, r := range live {
		if r.agent.comp == c && !r.ended {
			w.stamp(r.id, at)
		}
	}
}

func (w *world) eventually(what string, f func() (bool, string)) {
	w.t.Helper()
	var detail string
	for end := time.Now().Add(4 * time.Second); time.Now().Before(end); time.Sleep(25 * time.Millisecond) {
		ok, d := f()
		if ok {
			return
		}
		detail = d
	}
	w.t.Fatalf("%s: %s", what, detail)
}

// leaseOK reports whether tk's claim is held by r with leaseUntil equal to the ledger's expectation
// (within 1 s above it in real-clock mode, where send time precedes the server's renewal).
func (w *world) leaseOK(tk Task, r *run) (bool, string) {
	exp := w.expected(r)
	if tk.Claim == nil {
		return false, "task has no claim; expected one held by " + r.id
	}
	d := ms(tk.Claim.LeaseUntil) - exp
	if tk.Claim.RunID != r.id || (!realClock && d != 0) || (realClock && (d < -2 || d > 1000)) {
		return false, fmt.Sprintf("claim run=%s leaseUntil=%s, want run=%s leaseUntil=%d ms (last renewal + 60000), off by %d ms",
			tk.Claim.RunID, tk.Claim.LeaseUntil, r.id, exp, d)
	}
	return true, ""
}

func (w *world) checkLease(taskID string, r *run) {
	w.t.Helper()
	w.eventually("lease of "+taskID, func() (bool, string) { return w.leaseOK(w.task(taskID), r) })
}

func (w *world) tasks() []Task {
	var out struct {
		Tasks []Task `json:"tasks"`
	}
	w.must(w.c.do("GET", w.podPath("tasks"), w.owner.tok, nil, &out), "list tasks")
	return out.Tasks
}

func (w *world) task(id string) Task {
	for _, t := range w.tasks() {
		if t.ID == id {
			return t
		}
	}
	w.t.Fatalf("task %s is not in GET /v1/pods/:pod/tasks", id)
	return Task{}
}

func isType(typ string) func(Frame) bool { return func(f Frame) bool { return f.Type == typ } }

func (w *world) newTask(title, assignee string) Task {
	w.t.Helper()
	mk := w.sO.mark()
	data := map[string]any{"title": title, "kind": "chore"}
	if assignee != "" {
		data["assignee"] = assignee
	}
	w.sO.send("task.create", data)
	f, ok := w.sO.waitAfter(mk, func(f Frame) bool {
		var tk Task
		f.dec(&tk)
		return f.Type == "task.created" && tk.Title == title
	}, 5*time.Second)
	if !ok {
		w.t.Fatalf("task.create %q: no task.created within 5 s", title)
	}
	var tk Task
	f.dec(&tk)
	wantID(w.t, "tsk_", tk.ID)
	return tk
}

func wakeFor(agentID string) func(Frame) bool {
	return func(f Frame) bool {
		var wk Wake
		f.dec(&wk)
		return f.Type == "run.wake" && wk.AgentID == agentID
	}
}

// startRun assigns a task to the agent, takes its wake, acks it and claims the run, as a daemon would.
func (w *world) startRun(a *agent, title string) *run {
	w.t.Helper()
	mk := a.comp.sock.mark()
	w.newTask(title, a.id)
	f, ok := a.comp.sock.waitAfter(mk, wakeFor(a.id), 5*time.Second)
	if !ok {
		w.t.Fatalf("no run.wake for %s on its computer after a task was assigned to it", a.actor.DisplayName)
	}
	var wk Wake
	f.dec(&wk)
	wantID(w.t, "wak_", wk.WakeID)
	a.comp.sock.send("wake.ack", map[string]any{"wakeId": wk.WakeID})
	var out struct {
		RunID    string `json:"runId"`
		RunToken string `json:"runToken"`
	}
	w.must(w.c.do("POST", "/v1/runs", a.comp.tok, map[string]any{"wakeId": wk.WakeID}, &out), "POST /v1/runs")
	wantID(w.t, "run_", out.RunID)
	wantID(w.t, "jmr_", out.RunToken)
	r := &run{id: out.RunID, tok: out.RunToken, agent: a}
	w.mu.Lock()
	w.runs = append(w.runs, r)
	w.mu.Unlock()
	return r
}

func (w *world) claim(r *run, taskID string) res {
	return w.runCall(r, "POST", w.podPath("tasks", taskID, "claim"), map[string]any{}, nil)
}

func (w *world) mustClaim(r *run, taskID string) {
	w.t.Helper()
	w.must(w.claim(r, taskID), "claim "+taskID)
}

// raceClaim has both runs claim taskID at the same instant and returns the winner and the loser's result.
func (w *world) raceClaim(taskID string, a, b *run) (*run, res) {
	w.t.Helper()
	var rs [2]res
	gate := make(chan struct{})
	var wg sync.WaitGroup
	for i, r := range []*run{a, b} {
		wg.Add(1)
		go func(i int, r *run) {
			defer wg.Done()
			<-gate
			rs[i] = w.claim(r, taskID)
		}(i, r)
	}
	close(gate)
	wg.Wait()
	if rs[0].ok() && rs[1].Status == 409 {
		return a, rs[1]
	} else if rs[1].ok() && rs[0].Status == 409 {
		return b, rs[0]
	}
	w.t.Fatalf("concurrent claim of %s: want exactly one 2xx and one 409, got HTTP %d (%s) and HTTP %d (%s)",
		taskID, rs[0].Status, rs[0].Err.Code, rs[1].Status, rs[1].Err.Code)
	return nil, res{}
}
