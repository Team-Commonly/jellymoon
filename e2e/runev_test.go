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

// runevWatch is the pod's three member sockets, each marked before the run starts.
type runevWatch struct {
	w       *world
	members []*sock
	marks   []int
	r       *run
}

// eachGets waits for one `typ` frame for the run on every member socket and checks it with ok.
func (v *runevWatch) eachGets(typ string, ok func(Frame, runEv) bool) error {
	for i, s := range v.members {
		f, got := s.waitAfter(v.marks[i], runEvent(typ, v.r.id), 5*time.Second)
		if !got {
			return fmt.Errorf("%s: no %s for %s within 5 s", s.name, typ, v.r.id)
		}
		var d runEv
		f.dec(&d)
		if d.Agent.ID != v.w.a1.id || d.Agent.Kind != "agent" || !ok(f, d) {
			return fmt.Errorf("%s: %s %+v at seq %d is not what was asked for agent %s", s.name, typ, d, f.Seq, v.w.a1.id)
		}
	}
	return nil
}

func (v *runevWatch) stop(body map[string]any) error {
	if res := v.w.c.do("POST", "/v1/pods/"+v.w.pod+"/runs/"+v.r.id+"/stop", v.w.owner.tok, body, nil); !res.ok() {
		return fmt.Errorf("stop: HTTP %d code=%q", res.Status, res.Err.Code)
	}
	return nil
}

// onceEach: after a second stop, every member still holds exactly one run.started and one run.ended for the run.
func (v *runevWatch) onceEach() error {
	if err := v.stop(map[string]any{}); err != nil {
		return err
	}
	v.w.sO.none(v.w.sO.mark(), runEvent("run.ended", v.r.id), 500*time.Millisecond)
	for i, s := range v.members {
		if n, m := s.count(v.marks[i], runEvent("run.started", v.r.id)), s.count(v.marks[i], runEvent("run.ended", v.r.id)); n != 1 || m != 1 {
			return fmt.Errorf("%s: %d run.started and %d run.ended for one run, want 1 and 1", s.name, n, m)
		}
	}
	return nil
}

// replays: a member who subscribes from 0 gets run.started at startSeq, then run.ended.
func (v *runevWatch) replays(startSeq int64) error {
	s := v.w.openSince(v.w.ben.tok, "ben-replay", 0)
	a, okA := s.waitAfter(0, runEvent("run.started", v.r.id), 5*time.Second)
	b, okB := s.waitAfter(0, runEvent("run.ended", v.r.id), 5*time.Second)
	if !okA || !okB || a.Seq != startSeq || b.Seq <= a.Seq {
		return fmt.Errorf("replay: run.started %v (seq %d), run.ended %v (seq %d), want both with started at seq %d first", okA, a.Seq, okB, b.Seq, startSeq)
	}
	return nil
}

// strangerHearsNothing: the stranger's own socket got no run event, and the stranger cannot open the pod's socket.
func (w *world) strangerHearsNothing(sX *sock, stranger user) error {
	isRun := func(f Frame) bool { return f.Type == "run.started" || f.Type == "run.ended" }
	if !sX.none(0, isRun, 500*time.Millisecond) {
		return fmt.Errorf("the stranger's socket received a run event: %+v", sX.snapshot(0))
	}
	if conn, _, err := wsDial(fmt.Sprintf("127.0.0.1:%d", w.srv.port), "/v1/pods/"+w.pod+"/ws", stranger.tok); err == nil {
		conn.Close()
		return fmt.Errorf("a non-member opened the pod's socket")
	}
	return nil
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
	v := &runevWatch{w: w, members: []*sock{w.sO, w.sBen, w.sCara}}
	for _, s := range v.members {
		v.marks = append(v.marks, s.mark())
	}
	v.r = w.startRun(w.a1, "runev")
	var startSeq int64
	p.step("every member's socket receives run.started with the run and its agent", func() error {
		return v.eachGets("run.started", func(f Frame, _ runEv) bool { startSeq = f.Seq; return f.Seq > 0 })
	})
	p.step("the owner stops the run; every member receives one run.ended, outcome cancelled, after run.started", func() error {
		if err := v.stop(map[string]any{"reason": "from the web"}); err != nil {
			return err
		}
		return v.eachGets("run.ended", func(f Frame, d runEv) bool { return d.Outcome == "cancelled" && f.Seq > startSeq })
	})
	p.step("stopping it again emits no second run.ended", v.onceEach)
	p.step("a member who subscribes from 0 replays run.started then run.ended", func() error { return v.replays(startSeq) })
	p.step("a non-member receives neither event, and cannot open the pod's socket", func() error { return w.strangerHearsNothing(sX, stranger) })
	p.count(t)
	if p.ok != p.run {
		t.Fatalf("%d of %d steps passed", p.ok, p.run)
	}
}
