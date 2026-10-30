package e2e

import (
	"fmt"
	"os"
	"testing"
	"time"
)

// TestF06: a member creates a task assigned to a registered agent and every subscriber sees task.created.
func TestF06(t *testing.T) {
	needFake(t)
	w := newWorld(t)
	p := &stepper{out: os.Stdout, events: func() []Frame { return w.sCara.snapshot(0) }}
	p.step("a registered agent exists in the pod", func() error {
		h, err := firstHello(w.dial(w.owner.tok, "olivia-hello"))
		for _, a := range h.Agents {
			if a.Agent.ID == w.a1.id {
				return nil
			}
		}
		return fmt.Errorf("hello agents %+v do not list %s (err=%v)", h.Agents, w.a1.id, err)
	})
	subs := []*sock{w.sO, w.sBen, w.sCara}
	marks := map[*sock]int{}
	for _, s := range subs {
		marks[s] = s.mark()
	}
	p.step("a member sends task.create with the agent as assignee", func() error {
		w.sBen.send("task.create", map[string]any{"title": "assigned to one", "kind": "chore", "assignee": w.a1.id})
		return nil
	})
	p.step("all subscribers receive task.created with status open and the assignee set", func() error {
		var seq int64
		for _, s := range subs {
			f, ok := s.waitAfter(marks[s], isType("task.created"), 5*time.Second)
			if !ok {
				return fmt.Errorf("%s saw no task.created", s.name)
			}
			var tk Task
			f.dec(&tk)
			if tk.Title != "assigned to one" || tk.Status != "open" || tk.Assignee != w.a1.id || (seq != 0 && f.Seq != seq) {
				return fmt.Errorf("%s: seq %d task %+v, want status open, assignee %s and seq %d shared by all", s.name, f.Seq, tk, w.a1.id, seq)
			}
			seq = f.Seq
		}
		return nil
	})
	p.count()
	if p.ok != p.run {
		t.Fatalf("%d of %d steps passed", p.ok, p.run)
	}
}
