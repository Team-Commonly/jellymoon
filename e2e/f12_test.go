package e2e

import (
	"fmt"
	"os"
	"testing"
	"time"
)

// f12 is the world with one run that has posted an ask: a member rules on it, and the agent is woken with the ruling.
type f12 struct {
	*world
	first               *run
	taskID              string
	messageID, decision string
	ruleMark            map[*sock]int
	wakeMark            int
}

// ruledData is a message.updated frame: only the decision changes after a message is created.
type ruledData struct {
	ID       string
	Decision struct {
		ID, State string
		Ruling    *struct {
			OptionID string
			By       Actor
			At       string
		}
	}
}

type ruleWake struct {
	WakeID, AgentID, Reason string
	Refs                    struct{ TaskID, MessageID, DecisionID string }
}

func newF12(t *testing.T) *f12 {
	f := &f12{world: newWorld(t), ruleMark: map[*sock]int{}}
	f.first = f.startRun(f.a1, "decide the release")
	f.taskID = f.tasks()[0].ID
	f.wakeMark = f.compA.sock.mark()
	var out struct{ MessageID, DecisionID string }
	f.must(f.runCall(f.first, "POST", f.podPath("messages"), validAsk("Merge the auth fix now or wait?"), &out), "the run posts an ask")
	wantID(t, "msg_", out.MessageID)
	wantID(t, "dec_", out.DecisionID)
	f.messageID, f.decision = out.MessageID, out.DecisionID
	for _, s := range f.socks() {
		f.ruleMark[s] = s.mark()
	}
	return f
}

func (f *f12) socks() []*sock { return []*sock{f.sO, f.sBen, f.sCara} }

func (f *f12) ruling(s *sock, decisionID, option string) {
	s.send("decision.rule", map[string]any{"decisionId": decisionID, "optionId": option})
}

func isUpdated(messageID string) func(Frame) bool {
	return func(fr Frame) bool {
		var u ruledData
		fr.dec(&u)
		return fr.Type == "message.updated" && u.ID == messageID
	}
}

func errorWith(code string) func(Frame) bool {
	return func(fr Frame) bool {
		var e ErrorBody
		fr.dec(&e)
		return fr.Type == "error" && e.Code == code
	}
}

// refused: a ruling the pod will not take is answered on the sender's socket with its code, and nobody hears of a ruling.
func (f *f12) refused() error {
	cases := []struct {
		what, code string
		s          *sock
		dec, opt   string
	}{
		{"an option the ask does not offer", "invalid_input", f.sBen, f.decision, "c"},
		{"an option id the schema does not know", "invalid_input", f.sBen, f.decision, "z"},
		{"a decision that does not exist", "not_found", f.sBen, "dec_00000000000000000000000000", "a"},
		{"a computer's socket", "forbidden", f.compA.sock, f.decision, "a"},
	}
	for _, c := range cases {
		mk := c.s.mark()
		f.ruling(c.s, c.dec, c.opt)
		if _, ok := c.s.waitAfter(mk, errorWith(c.code), 5*time.Second); !ok {
			return fmt.Errorf("%s: no %s error on the sender's socket", c.what, c.code)
		}
	}
	for _, s := range f.socks() {
		if !s.none(f.ruleMark[s], isUpdated(f.messageID), 300*time.Millisecond) {
			return fmt.Errorf("%s heard of a ruling after only refused ones", s.name)
		}
	}
	return nil
}

// Step 1 and 2: a member rules once; every member sees the decision ruled, with who chose what and when.
func (f *f12) rules() error {
	f.ruling(f.sO, f.decision, "b")
	for _, s := range f.socks() {
		fr, ok := s.waitAfter(f.ruleMark[s], isUpdated(f.messageID), 5*time.Second)
		if !ok {
			return fmt.Errorf("%s got no message.updated for the ask within 5 s of the ruling", s.name)
		}
		var u ruledData
		fr.dec(&u)
		r := u.Decision.Ruling
		if u.Decision.ID != f.decision || u.Decision.State != "ruled" || r == nil || r.OptionID != "b" || r.By.ID != f.owner.actor.ID || !stamp.MatchString(r.At) {
			return fmt.Errorf("%s: message.updated %+v, want decision %s ruled with option b by %s and an RFC 3339 time", s.name, u, f.decision, f.owner.actor.ID)
		}
	}
	return nil
}

// Step 3: a second ruling, by anyone with any option, gets already_ruled; no member hears of it and the log keeps the first.
func (f *f12) rulesOnce() error {
	for _, c := range []struct {
		s   *sock
		opt string
	}{{f.sBen, "a"}, {f.sBen, "b"}, {f.sO, "a"}} {
		mk := c.s.mark()
		f.ruling(c.s, f.decision, c.opt)
		if _, ok := c.s.waitAfter(mk, errorWith("already_ruled"), 5*time.Second); !ok {
			return fmt.Errorf("a second ruling (%s, option %s) got no already_ruled error", c.s.name, c.opt)
		}
	}
	time.Sleep(200 * time.Millisecond)
	for _, s := range f.socks() {
		if n := s.count(f.ruleMark[s], isUpdated(f.messageID)); n != 1 {
			return fmt.Errorf("%s saw %d message.updated for the ask, want exactly the first ruling", s.name, n)
		}
	}
	replay := f.openSince(f.cara.tok, "cara-replay", 0)
	fr, ok := replay.waitAfter(0, isUpdated(f.messageID), 5*time.Second)
	time.Sleep(200 * time.Millisecond)
	var u ruledData
	fr.dec(&u)
	if !ok || replay.count(0, isUpdated(f.messageID)) != 1 || u.Decision.Ruling == nil || u.Decision.Ruling.OptionID != "b" || u.Decision.Ruling.By.ID != f.owner.actor.ID {
		return fmt.Errorf("a replay from the start holds %d rulings, the first %+v; want one, option b by the owner", replay.count(0, isUpdated(f.messageID)), u.Decision.Ruling)
	}
	return nil
}

// Step 4: the asking agent's computer gets one run.wake, reason decision_ruled, naming the decision and its message; the new run is stamped with it.
func (f *f12) woken() (*run, error) {
	fr, ok := f.compA.sock.waitAfter(f.wakeMark, wakeFor(f.a1.id), 5*time.Second)
	if !ok {
		return nil, fmt.Errorf("no run.wake for the asking agent after the ruling")
	}
	var wk ruleWake
	fr.dec(&wk)
	if wk.Reason != "decision_ruled" || wk.Refs.DecisionID != f.decision || wk.Refs.MessageID != f.messageID {
		return nil, fmt.Errorf("run.wake %+v, want reason decision_ruled with the decision %s and message %s", wk, f.decision, f.messageID)
	}
	if n := f.compA.sock.count(f.wakeMark, wakeFor(f.a1.id)); n != 1 {
		return nil, fmt.Errorf("the agent's computer got %d wakes for one ruling", n)
	}
	f.compA.sock.send("wake.ack", map[string]any{"wakeId": wk.WakeID})
	f.must(f.c.do("POST", "/v1/runs/"+f.first.id+"/end", f.compA.tok, map[string]any{"outcome": "completed"}, nil), "end the first run")
	f.first.ended = true
	var out struct{ RunID, RunToken string }
	f.must(f.c.do("POST", "/v1/runs", f.compA.tok, map[string]any{"wakeId": wk.WakeID}, &out), "POST /v1/runs for the decision_ruled wake")
	r := &run{id: out.RunID, tok: out.RunToken, agent: f.a1}
	f.mu.Lock()
	f.runs = append(f.runs, r)
	f.mu.Unlock()
	mk := f.sO.mark()
	f.must(f.runCall(r, "POST", f.podPath("messages"), map[string]any{"summary": "Merging the auth fix", "kind": "progress", "status": "in_progress"}, nil), "the woken run posts")
	fr, ok = f.sO.waitAfter(mk, isType("message.created"), 5*time.Second)
	var m struct{ Receipt *struct{ WakeReason, RunID string } }
	fr.dec(&m)
	if !ok || m.Receipt == nil || m.Receipt.WakeReason != "decision_ruled" || m.Receipt.RunID != r.id {
		return nil, fmt.Errorf("the woken run's post carries receipt %+v, want wakeReason decision_ruled on run %s", m.Receipt, r.id)
	}
	return r, nil
}

// Step 5: the agent finishes the task over MCP, and members see it done.
func (f *f12) finishes(r *run) error {
	marks := map[*sock]int{}
	for _, s := range f.socks() {
		marks[s] = s.mark()
	}
	call := map[string]any{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": map[string]any{"name": "jm_task_update", "arguments": map[string]any{"taskId": f.taskID, "status": "done"}}}
	var out struct{ Result struct{ IsError bool } }
	if rs := f.c.do("POST", "/mcp", r.tok, call, &out); !rs.ok() || out.Result.IsError {
		return fmt.Errorf("jm_task_update over MCP: HTTP %d, isError %v", rs.Status, out.Result.IsError)
	}
	for _, s := range f.socks() {
		fr, ok := s.waitAfter(marks[s], func(fr Frame) bool {
			var tk Task
			fr.dec(&tk)
			return fr.Type == "task.updated" && tk.ID == f.taskID && tk.Status == "done"
		}, 5*time.Second)
		if !ok {
			return fmt.Errorf("%s got no task.updated done for %s (frame %s)", s.name, f.taskID, fr.Data)
		}
	}
	return nil
}

// TestF12: a member rules on an ask once, the agent is woken with the ruling and finishes the task.
func TestF12(t *testing.T) {
	needFake(t)
	f := newF12(t)
	p := &stepper{out: os.Stdout, events: func() []Frame { return f.sO.snapshot(0) }}
	defer p.count(t)
	var next *run
	p.step("a ruling the pod refuses is answered with its code and leaves the ask open", f.refused)
	p.step("a member sends decision.rule with an option id; members receive message.updated with the decision ruled and frozen", f.rules)
	p.step("a second ruling returns already_ruled and changes nothing", f.rulesOnce)
	p.step("the agent receives run.wake with reason decision_ruled", func() (err error) { next, err = f.woken(); return err })
	p.step("the agent calls jm_task_update done and members receive task.updated done", func() error {
		if next == nil {
			return fmt.Errorf("there is no run: the wake step failed")
		}
		return f.finishes(next)
	})
	if p.ok != p.run {
		t.Fatalf("%d of %d steps passed", p.ok, p.run)
	}
}
