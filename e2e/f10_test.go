package e2e

import (
	"strings"
	"testing"
	"time"
)

var causes = []string{"expired", "released", "run_ended", "stopped"}

// wantErr fails the test unless rs is the given HTTP status and error code.
func (w *world) wantErr(rs res, status int, code, what string) {
	w.t.Helper()
	if rs.Status != status || rs.Err.Code != code {
		w.t.Errorf("%s: HTTP %d %s, want %d %s", what, rs.Status, rs.Err.Code, status, code)
	}
}

func (w *world) podPath(parts ...string) string {
	return "/v1/pods/" + w.pod + "/" + strings.Join(parts, "/")
}

// TestF10: claims are exclusive, the loser is told the holder, leases follow the run, expiry and
// release behave, and every way a claim ends sends one claim.released.
func TestF10(t *testing.T) {
	needFake(t)
	w := newWorld(t)
	r1, r2 := w.startRun(w.a1, "wake-1"), w.startRun(w.a2, "wake-2")
	win := w.f10Race(r1, r2)
	w.f10Renewal(win)
	w.f10Expiry(win)
	w.f10Release(win)
	for _, cause := range causes { // step 7
		w.scenario(cause)
	}
}

// Steps 1 to 3: exactly one winner, the loser names the holder, members get task.claimed.
func (w *world) f10Race(r1, r2 *run) *run {
	c := w.newTask("contested", "")
	mkO, mkB := w.sO.mark(), w.sBen.mark()
	win, lr := w.raceClaim(c.ID, r1, r2)
	holder := lr.Err.Ref + " " + lr.Err.Message
	if !strings.Contains(holder, win.id) && !strings.Contains(holder, win.agent.id) && !strings.Contains(holder, win.agent.actor.DisplayName) {
		w.t.Errorf("already_claimed does not name the holder (run %s, agent %s): ref=%q message=%q", win.id, win.agent.id, lr.Err.Ref, lr.Err.Message)
	}
	for s, mk := range map[*sock]int{w.sO: mkO, w.sBen: mkB} {
		var cl Claim
		w.expect(s, mk, isType("task.claimed"), "task.claimed").dec(&cl)
		if cl.TaskID != c.ID || cl.RunID != win.id || cl.By.ID != win.agent.id || ms(cl.LeaseUntil) != w.expected(win) {
			w.t.Errorf("%s: task.claimed %+v, want task %s run %s agent %s leaseUntil %d", s.name, cl, c.ID, win.id, win.agent.id, w.expected(win))
		}
	}
	w.contested = c.ID
	w.checkLease(c.ID, win)
	return win
}

// Step 4: an operation on the run, or a heartbeat from its computer, moves leaseUntil forward; a user's read does not.
func (w *world) f10Renewal(win *run) {
	w.advance(20000)
	w.must(w.runCall(win, "GET", w.podPath("me"), nil, nil), "whoami on the run token")
	w.checkLease(w.contested, win)
	w.advance(10000)
	w.task(w.contested)
	w.checkLease(w.contested, win)
	w.heartbeat(w.compA)
	w.checkLease(w.contested, win)
}

// Step 5: with the daemon stopped the claim is dropped 60 s after the last renewal, then restored by its run.
func (w *world) f10Expiry(win *run) {
	w.compA.sock.close()
	exp := w.expected(win)
	w.setClock(exp - 1)
	w.checkLease(w.contested, win)
	mk := w.sBen.mark()
	w.setClock(exp)
	if _, ok := w.awaitExpiry(w.sBen, mk, w.contested, win, 5*time.Second); !ok {
		w.t.FailNow()
	}
	w.must(w.runCall(win, "PATCH", w.podPath("tasks", w.contested), map[string]any{"status": "in_progress"}, nil), "task_update after expiry")
	w.checkLease(w.contested, win)
}

// Step 6: the claiming agent's owner releases; the run is then forbidden and nothing restores the claim.
func (w *world) f10Release(win *run) {
	d := w.newTask("release-me", "")
	w.mustClaim(win, d.ID)
	rel := w.podPath("tasks", d.ID, "release")
	w.wantErr(w.c.do("POST", rel, w.cara.tok, map[string]any{}, nil), 403, "forbidden", "release by a member who is neither holder, agent owner nor pod owner")
	mk := w.sO.mark()
	w.must(w.c.do("POST", rel, w.ben.tok, map[string]any{"reason": "taking it back"}, nil), "release by the agent's owner")
	var rl Released
	w.expect(w.sO, mk, released(d.ID, "released"), "claim.released").dec(&rl)
	if rl.By == nil || rl.By.ID != w.ben.actor.ID || rl.Reason != "taking it back" {
		w.t.Errorf("claim.released after a release: %+v, want by ben and the reason", rl)
	}
	w.expect(w.sO, mk, unclaimed(d.ID), "task.updated without a claim")
	w.wantErr(w.runCall(win, "PATCH", w.podPath("tasks", d.ID), map[string]any{"status": "in_progress"}, nil), 403, "forbidden", "task_update by the run whose claim was released")
}

// TestOneReleasePerCause: for each cause, every member gets exactly one claim.released with that cause.
func TestOneReleasePerCause(t *testing.T) {
	needFake(t)
	w := newWorld(t)
	for _, cause := range causes {
		w.scenario(cause)
	}
}

// scenario ends one fresh claim by cause and checks what each member (and the computer) sees.
func (w *world) scenario(cause string) {
	w.t.Helper()
	if w.compA.sock.isDead() {
		w.compA.sock = w.open(w.compA.tok, "computer-a-again")
	}
	r := w.startRun(w.a1, "wake-"+cause)
	tk := w.newTask("work-"+cause, "")
	w.mustClaim(r, tk.ID)
	socks := []*sock{w.sO, w.sBen, w.sCara}
	marks := []int{w.sO.mark(), w.sBen.mark(), w.sCara.mark()}
	cm := w.compA.sock.mark()
	var by *Actor
	reason := ""
	switch cause {
	case "expired":
		w.setClock(w.expected(r))
	case "released":
		by, reason = &w.ben.actor, "handing back"
		w.must(w.c.do("POST", w.podPath("tasks", tk.ID, "release"), w.ben.tok, map[string]any{"reason": reason}, nil), "release")
	case "run_ended":
		w.must(w.c.do("POST", "/v1/runs/"+r.id+"/end", w.compA.tok, map[string]any{"outcome": "completed"}, nil), "end run")
	case "stopped":
		by, reason = &w.owner.actor, "stop now"
		w.must(w.c.do("POST", w.podPath("runs", r.id, "stop"), w.owner.tok, map[string]any{"reason": reason}, nil), "stop run")
		w.expect(w.compA.sock, cm, func(f Frame) bool { var s Released; f.dec(&s); return f.Type == "run.stop" && s.RunID == r.id }, "run.stop on the hosting computer")
		w.wantErr(w.c.do("GET", w.podPath("me"), r.tok, nil, nil), 401, "unauthorized", "the stopped run's token")
	}
	r.ended = cause == "run_ended" || cause == "stopped"
	for i, s := range socks {
		var rl Released
		w.expect(s, marks[i], released(tk.ID, cause), "claim.released "+cause).dec(&rl)
		if rl.RunID != r.id || (by == nil) != (rl.By == nil) || (by != nil && rl.By.ID != by.ID) || rl.Reason != reason {
			w.t.Errorf("%s/%s: claim.released %+v, want run %s by %v reason %q", cause, s.name, rl, r.id, by, reason)
		}
	}
	w.advance(120000)
	time.Sleep(300 * time.Millisecond)
	for i, s := range socks {
		w.oneRelease(s, marks[i], tk.ID, 1)
	}
}
