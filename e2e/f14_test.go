package e2e

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
	"time"
)

// f14 is the whole slice, walked the way a person would: F09's world (a built daemon, the fake ACP agent, a recording proxy) with
// nobody signed in yet. The agent runs a script twice: once to claim the task and ask, once, after the ruling, to finish. The task id
// is not known when the script is written, so the script says {{taskId}} and the fake agent reads it from its brief. The first script is
// written once the pod holds the CI report its ask cites (F25), before the task that wakes the agent.
type f14 struct {
	*f09
	begin   time.Time
	record  string
	script  string
	taskID  string
	first   claimed
	ask     postedMsg
	decide  string
	secondW claimed
}

func writeScript(path string, steps ...map[string]any) error {
	raw, _ := json.Marshal(steps)
	tmp := path + ".tmp"
	if err := os.WriteFile(tmp, raw, 0o600); err != nil {
		return err
	}
	return os.Rename(tmp, path)
}

// withTask attaches the ask to the task its wake named, the way an agent working a task would.
func withTask(ask map[string]any) map[string]any {
	ask["taskId"] = "{{taskId}}"
	return ask
}

func newF14(t *testing.T) *f14 {
	f := &f14{f09: newF09On(newF08Bare(t)), begin: time.Now()}
	f.script, f.record = filepath.Join(t.TempDir(), "script.json"), filepath.Join(t.TempDir(), "agent.json")
	f.harness(f.record, "script="+f.script)
	return f
}

// nth waits for the n-th (from 1) request whose method and path match.
func (r *recorder) nth(method string, path *regexp.Regexp, n int, d time.Duration) (hit, bool) {
	for end := time.Now().Add(d); time.Now().Before(end); time.Sleep(25 * time.Millisecond) {
		r.mu.Lock()
		seen := 0
		for _, h := range r.hits {
			if h.Method == method && path.MatchString(h.Path) {
				if seen++; seen == n {
					r.mu.Unlock()
					return h, true
				}
			}
		}
		r.mu.Unlock()
	}
	return hit{}, false
}

// both waits for the owner's and ben's sockets to each receive a frame satisfying pred after their marks.
func (f *f14) both(marks []int, what string, pred func(Frame) bool) (Frame, error) {
	var last Frame
	for i, s := range f.obs {
		fr, ok := s.waitAfter(marks[i], pred, 5*time.Second)
		if !ok {
			return fr, fmt.Errorf("%s saw no %s within 5 s", s.name, what)
		}
		last = fr
	}
	return last, nil
}

// signsIn is how a person gets a token (F15): the browser leg through the sign-in provider, which approves a device code, and the
// token the device flow exchanges for it. The stub's authorize hop is played here with this person's own identity, so the parallel
// TestF15 steering the stub's next login cannot touch it (as TestBSess does).
func (f *f14) signsIn(login string, id int64) (user, error) {
	hc, base := browser(), f.srv.base()
	resp, err := hc.Get(base + "/v1/auth/github/start?next=/")
	if err != nil || resp.StatusCode != 302 {
		return user{}, fmt.Errorf("%s: GET /v1/auth/github/start: %v %v, want 302", login, err, resp)
	}
	q, _ := url.Parse(resp.Header.Get("Location"))
	code, tok := randHex(10), "gho_stub"+randHex(18)
	gh.mu.Lock()
	gh.codes[code], gh.tokens[tok], gh.users[tok] = tok, true, ghUser{ID: id, Login: login}
	gh.mu.Unlock()
	cb := q.Query().Get("redirect_uri") + "?" + url.Values{"code": {code}, "state": {q.Query().Get("state")}}.Encode()
	if resp, err = hc.Get(cb); err != nil || resp.StatusCode != 302 {
		return user{}, fmt.Errorf("%s: the sign-in callback: %v %v, want 302 with a session", login, err, resp)
	}
	var d deviceStart
	if r := f.c.do("POST", "/v1/auth/device", "", nil, &d); !r.ok() {
		return user{}, fmt.Errorf("%s: POST /v1/auth/device: HTTP %d %s", login, r.Status, r.Err.Code)
	}
	req, _ := http.NewRequest("POST", base+"/v1/auth/device/approve", strings.NewReader(fmt.Sprintf(`{"userCode":%q}`, d.UserCode)))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Origin", base)
	if resp, err = hc.Do(req); err != nil || resp.StatusCode != 200 {
		return user{}, fmt.Errorf("%s: approve the device code: %v %v, want 200", login, err, resp)
	}
	var out deviceToken
	if r := f.c.do("POST", "/v1/auth/device/token", "", map[string]any{"deviceCode": d.DeviceCode}, &out); !r.ok() || !strings.HasPrefix(out.Token, "jmu_") || out.User.DisplayName != login {
		return user{}, fmt.Errorf("%s: the device token: HTTP %d %q user %+v, want a jmu_ token for %s", login, r.Status, r.Err.Code, out.User, login)
	}
	return user{tok: out.Token, actor: out.User}, nil
}

// Step 1: two people sign in, one creates the pod and invites, the other joins; each is a member, as hello says.
func (f *f14) joins() error {
	var err error
	if f.owner, err = f.signsIn("olivia", 4200001); err != nil {
		return err
	}
	if f.ben, err = f.signsIn("ben", 4200002); err != nil {
		return err
	}
	f.joinPod()
	conn, br, err := wsDial(fmt.Sprintf("127.0.0.1:%d", f.srv.port), "/v1/pods/"+f.pod+"/ws", f.ben.tok)
	if err != nil {
		return err
	}
	s := newSock("ben-hello", f.pod, conn, br)
	defer s.close()
	h, err := firstHello(s)
	if err != nil {
		return err
	}
	roles := map[string]string{}
	for _, m := range h.Members {
		roles[m.Actor.ID] = m.Role
	}
	if h.PodID != f.pod || h.You.ID != f.ben.actor.ID || roles[f.owner.actor.ID] != "owner" || roles[f.ben.actor.ID] != "member" || len(roles) != 2 {
		return fmt.Errorf("ben's hello: pod %q you %q members %v, want pod %s with olivia owner and ben member", h.PodID, h.You.ID, roles, f.pod)
	}
	return nil
}

// Step 2: the daemon registers its computer, the fake agent is added, and both people see it online.
func (f *f14) agentOnline() error {
	if err := f.registersComputer(); err != nil {
		return err
	}
	if err := f.addsAgent(); err != nil {
		return err
	}
	return f.all("online", []int{0, 0}, 5*time.Second)
}

// Step 3: a person creates a task assigned to the agent, and both people see it open, with the agent as assignee. Before it, the person
// posts the CI report the agent's ask will cite.
func (f *f14) createsTask() error {
	cited, err := ciReport(f.c, f.pod, f.owner.tok)
	if err != nil {
		return err
	}
	if err := writeScript(f.script,
		step11("jm_task_list", map[string]any{}),
		step11("jm_task_claim", map[string]any{"taskId": "{{taskId}}"}),
		step11("jm_post", withTask(validAsk("Merge the auth fix now or wait for review?", cited))),
	); err != nil {
		return err
	}
	marks := f.marks()
	f.assign("Ship the auth fix")
	fr, err := f.both(marks, "task.created for the assigned task", func(fr Frame) bool {
		var tk Task
		fr.dec(&tk)
		return fr.Type == "task.created" && tk.Title == "Ship the auth fix"
	})
	if err != nil {
		return err
	}
	var tk Task
	fr.dec(&tk)
	if !strings.HasPrefix(tk.ID, "tsk_") || tk.Status != "open" || tk.Assignee != f.agentID {
		return fmt.Errorf("task.created %+v, want a tsk_ id, status open, assignee %s", tk, f.agentID)
	}
	f.taskID = tk.ID
	return nil
}

// Step 4a: the daemon claims the wake the assignment made (it is the one that sees run.wake), and the agent's brief names the task.
func (f *f14) claimsWake() error {
	h, ok := f.px.find("POST", runPath, 10*time.Second)
	if !ok || h.Status != 200 || h.Auth != "Bearer "+f.comp.Token {
		return fmt.Errorf("the daemon never claimed the wake with POST /v1/runs (found=%v HTTP %d)\ndaemon output:\n%s", ok, h.Status, f.daemon.text())
	}
	_ = json.Unmarshal(h.Out, &f.first)
	f.run = f.first
	if err := f.response(); err != nil {
		return err
	}
	if !strings.Contains(f.first.Brief, f.taskID) || !strings.Contains(f.first.Brief, "task_assigned") {
		return fmt.Errorf("the first brief %q does not name task %s and task_assigned", f.first.Brief, f.taskID)
	}
	return nil
}

// Step 4b: the agent claims the task over MCP, and both people see task.claimed with its run and its lease.
func (f *f14) taskClaimed() error {
	fr, err := f.both([]int{0, 0}, "task.claimed for the task", func(fr Frame) bool {
		var c Claim
		fr.dec(&c)
		return fr.Type == "task.claimed" && c.TaskID == f.taskID
	})
	if err != nil {
		return err
	}
	var c Claim
	fr.dec(&c)
	if c.RunID != f.first.RunID || c.By.ID != f.agentID || ms(c.LeaseUntil) != f.clock+leaseMs {
		return fmt.Errorf("task.claimed %+v, want run %s by agent %s with the lease 60 s past the fake clock", c, f.first.RunID, f.agentID)
	}
	return nil
}

func isAsk(fr Frame) bool {
	var m postedMsg
	fr.dec(&m)
	return fr.Type == "message.created" && m.Kind == "ask"
}

// Step 4c: the agent's ask reaches both people with an open decision and the pod's receipt, that counts the real tool calls.
func (f *f14) askWithReceipt() error {
	fr, err := f.both([]int{0, 0}, "the agent's ask", isAsk)
	if err != nil {
		return err
	}
	fr.dec(&f.ask)
	m, r := f.ask, f.ask.Receipt
	if m.Status != "blocked" || m.Author.ID != f.agentID || m.RunID != f.first.RunID || m.Decision == nil || m.Decision.State != "open" || !decisionID.MatchString(m.Decision.ID) || r == nil {
		return fmt.Errorf("the ask %+v: want blocked, by the agent on run %s, with an open dec_ decision and a receipt", m, f.first.RunID)
	}
	f.decide = m.Decision.ID
	if r.RunID != f.first.RunID || r.AgentID != f.agentID || r.ComputerID != f.comp.ComputerID || r.Harness != "fake" || r.WakeReason != "task_assigned" || r.TaskID != f.taskID {
		return fmt.Errorf("receipt %+v, want run %s agent %s computer %s harness fake wakeReason task_assigned task %s", *r, f.first.RunID, f.agentID, f.comp.ComputerID, f.taskID)
	}
	return f.receiptCounts(r.ToolCalls)
}

// receiptCounts: the receipt's count is the MCP calls the agent made before the ask, as the agent itself recorded them (each answered, none refused).
func (f *f14) receiptCounts(counted int) error {
	rec, err := f.recorded(f.record, func(r agentRec) bool { return len(r.Calls) == 3 })
	if err != nil {
		return err
	}
	var tools []string
	for _, c := range rec.Calls[:2] {
		var env struct{ Result struct{ IsError bool } }
		if json.Unmarshal(c.Reply, &env) != nil || env.Result.IsError || c.Error != "" {
			return fmt.Errorf("the agent's call %s was refused or unreadable: %s %s", c.Tool, c.Reply, c.Error)
		}
		tools = append(tools, c.Tool)
	}
	if got := strings.Join(tools, ","); got != "jm_task_list,jm_task_claim" || counted != len(tools) {
		return fmt.Errorf("the agent made %s before its ask and the receipt counts %d; want jm_task_list,jm_task_claim and 2", got, counted)
	}
	return nil
}

// Step 5a: the first run ends, then a member rules; both people see the decision ruled, with who chose what.
func (f *f14) rules() error {
	h, ok := f.px.find("POST", regexp.MustCompile(`^/v1/runs/`+f.first.RunID+`/end$`), 10*time.Second)
	if !ok || h.Status != 200 || !strings.Contains(string(h.In), `"completed"`) {
		return fmt.Errorf("the first run was not ended completed: found=%v HTTP %d body %s", ok, h.Status, h.In)
	}
	// The agent has read its script by now, so the second run gets the finishing script. No task id is written in: the ruling's wake
	// names the task the ask was attached to, so the brief of the second run carries a tsk_ id for the script to borrow.
	if err := writeScript(f.script,
		step11("jm_post", map[string]any{"summary": "Merged the auth fix", "kind": "result", "status": "done"}),
		step11("jm_task_update", map[string]any{"taskId": "{{taskId}}", "status": "done"}),
	); err != nil {
		return err
	}
	marks := f.marks()
	f.obs[0].send("decision.rule", map[string]any{"decisionId": f.decide, "optionId": "a"})
	fr, err := f.both(marks, "message.updated for the ask", isUpdated(f.ask.ID))
	if err != nil {
		return err
	}
	var u ruledData
	fr.dec(&u)
	if r := u.Decision.Ruling; u.Decision.State != "ruled" || r == nil || r.OptionID != "a" || r.By.ID != f.owner.actor.ID {
		return fmt.Errorf("message.updated %+v, want the decision ruled with option a by %s", u, f.owner.actor.ID)
	}
	return nil
}

// Step 5b: the ruling wakes the agent a second time: the daemon claims a new run whose brief says why.
func (f *f14) secondWake() error {
	h, ok := f.px.nth("POST", runPath, 2, 10*time.Second)
	if !ok || h.Status != 200 {
		return fmt.Errorf("the daemon never claimed a second wake (found=%v HTTP %d)\ndaemon output:\n%s", ok, h.Status, f.daemon.text())
	}
	_ = json.Unmarshal(h.Out, &f.secondW)
	if f.secondW.RunID == "" || f.secondW.RunID == f.first.RunID || !strings.Contains(f.secondW.Brief, "decision_ruled") || !strings.Contains(f.secondW.Brief, f.decide) ||
		!strings.Contains(f.secondW.Brief, "taskId "+f.taskID) || !strings.Contains(f.secondW.Brief, "optionId a") {
		return fmt.Errorf("second claim: run %q (first %q), brief %q; want a new run whose brief names decision_ruled, %s, taskId %s and optionId a", f.secondW.RunID, f.first.RunID, f.secondW.Brief, f.decide, f.taskID)
	}
	return nil
}

// Step 5c: the woken agent reports and finishes the task: its result carries the second wake's receipt, and both people see task.updated done.
func (f *f14) finishes() error {
	marks := []int{0, 0}
	fr, err := f.both(marks, "the agent's result", func(fr Frame) bool {
		var m postedMsg
		fr.dec(&m)
		return fr.Type == "message.created" && m.Kind == "result" && m.Summary == "Merged the auth fix"
	})
	if err != nil {
		return err
	}
	var m postedMsg
	fr.dec(&m)
	if r := m.Receipt; r == nil || r.RunID != f.secondW.RunID || r.WakeReason != "decision_ruled" || r.AgentID != f.agentID {
		return fmt.Errorf("the result's receipt %+v, want run %s, agent %s, wakeReason decision_ruled", r, f.secondW.RunID, f.agentID)
	}
	fr, err = f.both(marks, "task.updated done for the task", func(fr Frame) bool {
		var tk Task
		fr.dec(&tk)
		return fr.Type == "task.updated" && tk.ID == f.taskID && tk.Status == "done"
	})
	if err != nil {
		raw, _ := os.ReadFile(f.record)
		return fmt.Errorf("%v; the second run's agent recorded %s", err, raw)
	}
	h, ok := f.px.find("POST", regexp.MustCompile(`^/v1/runs/`+f.secondW.RunID+`/end$`), 10*time.Second)
	if !ok || h.Status != 200 || !strings.Contains(string(h.In), `"completed"`) {
		return fmt.Errorf("the second run was not ended completed: found=%v HTTP %d body %s (last frame %s)", ok, h.Status, h.In, fr.Data)
	}
	return nil
}

// TestF14: the whole slice end to end with the fake agent, as one walk.
func TestF14(t *testing.T) {
	needFake(t)
	par(t)
	track(t)
	f := newF14(t)
	p := &stepper{out: os.Stdout, events: func() []Frame { return f.obs[0].snapshot(0) }}
	defer p.count(t)
	p.step("sign in, create a pod, invite, join", f.joins)
	p.step("start the daemon and add the fake agent", f.agentOnline)
	p.step("create a task assigned to the agent", f.createsTask)
	p.step("the agent's wake is claimed and its brief names the task", f.claimsWake)
	p.step("the agent claims the task and members receive task.claimed", f.taskClaimed)
	p.step("the agent's ask reaches members with a receipt that counts the tool calls the agent made", f.askWithReceipt)
	p.step("a member rules on the ask and members receive the ruling", f.rules)
	p.step("the ruling wakes the agent a second time with reason decision_ruled", f.secondWake)
	p.step("the agent posts its result with the new receipt and members receive task.updated done", f.finishes)
	p.step("the walk took under 60 s", func() error {
		if d := time.Since(f.begin); d >= 60*time.Second {
			return fmt.Errorf("the walk took %v, want under 60 s", d)
		}
		return nil
	})
	if p.ok != p.run {
		t.Fatalf("%d of %d steps passed", p.ok, p.run)
	}
}
