package e2e

import (
	"fmt"
	"os"
	"testing"
	"time"
)

type runEv struct {
	RunID   string `json:"runId"`
	Agent   Actor  `json:"agent"`
	Outcome string `json:"outcome"`
}

// runEvent matches a run.started or run.ended frame for one run.
func runEvent(typ, runID string) func(Frame) bool {
	return func(f Frame) bool {
		var d runEv
		f.dec(&d)
		return f.Type == typ && d.RunID == runID
	}
}

// strangerSock is a non-member of the world's pod, subscribed on a pod of their own: every event they may see arrives here.
func (w *world) strangerSock(u user) *sock {
	var pod struct {
		ID string `json:"id"`
	}
	w.must(w.c.do("POST", "/v1/pods", u.tok, map[string]any{"name": "elsewhere"}, &pod), "stranger creates a pod")
	conn, br, err := wsDial(fmt.Sprintf("127.0.0.1:%d", w.srv.port), "/v1/pods/"+pod.ID+"/ws", u.tok)
	if err != nil {
		w.t.Fatalf("stranger: WebSocket on their own pod: %v", err)
	}
	s := newSock("stranger", pod.ID, conn, br)
	w.t.Cleanup(s.close)
	if _, ok := s.waitAfter(0, isType("hello"), 5*time.Second); !ok {
		w.t.Fatalf("stranger: no hello on their own pod")
	}
	s.send("subscribe", map[string]any{"sinceSeq": 0})
	return s
}

// TestRUNEV: members hear run.started when a run begins and run.ended (cancelled) once when a person stops it, both seq'd and
// replayed on subscribe; a non-member hears neither and cannot open the pod's socket.
func TestRUNEV(t *testing.T) {
	needFake(t)
	par(t)
	w := newWorld(t)
	p := &stepper{out: os.Stdout, events: func() []Frame { return w.sO.snapshot(0) }}
	stranger := w.session("stranger")
	sX := w.strangerSock(stranger)
	members := []*sock{w.sO, w.sBen, w.sCara}
	marks := make([]int, len(members))
	for i, s := range members {
		marks[i] = s.mark()
	}
	r := w.startRun(w.a1, "runev")
	var startSeq int64
	p.step("every member's socket receives run.started with the run and its agent", func() error {
		for i, s := range members {
			f, ok := s.waitAfter(marks[i], runEvent("run.started", r.id), 5*time.Second)
			if !ok {
				return fmt.Errorf("%s: no run.started for %s within 5 s", s.name, r.id)
			}
			var d runEv
			f.dec(&d)
			if d.Agent.ID != w.a1.id || d.Agent.Kind != "agent" || f.Seq == 0 {
				return fmt.Errorf("%s: run.started agent %+v seq %d, want agent %s and a seq", s.name, d.Agent, f.Seq, w.a1.id)
			}
			startSeq = f.Seq
		}
		return nil
	})
	p.step("the owner stops the run; every member receives one run.ended, outcome cancelled, after run.started", func() error {
		if res := w.c.do("POST", "/v1/pods/"+w.pod+"/runs/"+r.id+"/stop", w.owner.tok, map[string]any{"reason": "from the web"}, nil); !res.ok() {
			return fmt.Errorf("stop: HTTP %d code=%q", res.Status, res.Err.Code)
		}
		for i, s := range members {
			f, ok := s.waitAfter(marks[i], runEvent("run.ended", r.id), 5*time.Second)
			if !ok {
				return fmt.Errorf("%s: no run.ended for %s within 5 s", s.name, r.id)
			}
			var d runEv
			f.dec(&d)
			if d.Outcome != "cancelled" || d.Agent.ID != w.a1.id || f.Seq <= startSeq {
				return fmt.Errorf("%s: run.ended %+v seq %d, want cancelled for %s after seq %d", s.name, d, f.Seq, w.a1.id, startSeq)
			}
		}
		return nil
	})
	p.step("stopping it again emits no second run.ended", func() error {
		if res := w.c.do("POST", "/v1/pods/"+w.pod+"/runs/"+r.id+"/stop", w.owner.tok, map[string]any{}, nil); !res.ok() {
			return fmt.Errorf("second stop: HTTP %d code=%q", res.Status, res.Err.Code)
		}
		w.sO.none(w.sO.mark(), runEvent("run.ended", r.id), 500*time.Millisecond)
		for i, s := range members {
			if n, m := s.count(marks[i], runEvent("run.started", r.id)), s.count(marks[i], runEvent("run.ended", r.id)); n != 1 || m != 1 {
				return fmt.Errorf("%s: %d run.started and %d run.ended for one run, want 1 and 1", s.name, n, m)
			}
		}
		return nil
	})
	p.step("a member who subscribes from 0 replays run.started then run.ended", func() error {
		s := w.openSince(w.ben.tok, "ben-replay", 0)
		a, okA := s.waitAfter(0, runEvent("run.started", r.id), 5*time.Second)
		b, okB := s.waitAfter(0, runEvent("run.ended", r.id), 5*time.Second)
		if !okA || !okB || a.Seq != startSeq || b.Seq <= a.Seq {
			return fmt.Errorf("replay: run.started %v (seq %d), run.ended %v (seq %d), want both with started at seq %d first", okA, a.Seq, okB, b.Seq, startSeq)
		}
		return nil
	})
	p.step("a non-member receives neither event, and cannot open the pod's socket", func() error {
		isRun := func(f Frame) bool { return f.Type == "run.started" || f.Type == "run.ended" }
		if !sX.none(0, isRun, 500*time.Millisecond) {
			return fmt.Errorf("the stranger's socket received a run event: %+v", sX.snapshot(0))
		}
		if conn, _, err := wsDial(fmt.Sprintf("127.0.0.1:%d", w.srv.port), "/v1/pods/"+w.pod+"/ws", stranger.tok); err == nil {
			conn.Close()
			return fmt.Errorf("a non-member opened the pod's socket")
		}
		return nil
	})
	p.count(t)
	if p.ok != p.run {
		t.Fatalf("%d of %d steps passed", p.ok, p.run)
	}
}
