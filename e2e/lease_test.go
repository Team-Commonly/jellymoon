package e2e

import (
	"fmt"
	"sync"
	"testing"
	"time"
)

// TestExpiryZeroTraffic: a claim expires on its own. After the last renewal the suite sends no run
// operation and no heartbeat; only the clock moves (fake mode) or time passes (real-clock mode).
func TestExpiryZeroTraffic(t *testing.T) {
	need(t)
	if realClock {
		t.Parallel()
	}
	w := newWorld(t)
	r := w.startRun(w.a1, "wake")
	tk := w.newTask("lease", "")
	w.mustClaim(r, tk.ID)
	w.checkLease(tk.ID, r)
	w.compA.sock.close()
	exp := w.expected(r)
	mk := w.sBen.mark()
	wait := 5 * time.Second
	if realClock {
		wait = time.Until(time.UnixMilli(exp)) + 15*time.Second
	} else {
		w.setClock(exp - 1)
		w.checkLease(tk.ID, r)
		w.noRelease(w.sBen, tk.ID, 300*time.Millisecond)
		w.setClock(exp)
	}
	obs, ok := w.awaitExpiry(w.sBen, mk, tk.ID, r, wait)
	if !ok {
		t.FailNow()
	}
	if realClock {
		late := obs - exp
		fmt.Printf("LATENESS_MS steady %d\n", late)
		if late < -2 || late > 6000 {
			t.Fatalf("lateness %d ms outside [-2, 6000]: early expiry or an alarm that fired late", late)
		}
	} else {
		w.advance(120000)
	}
	time.Sleep(300 * time.Millisecond)
	w.oneRelease(w.sBen, mk, tk.ID, 1)
}

// TestRenewRace: two runs race for claims while renewals and heartbeats land. Never two holders; a lease
// only moves forward; it always equals the last recorded renewal plus 60 s.
func TestRenewRace(t *testing.T) {
	needFake(t)
	w := newWorld(t)
	r1, r2 := w.startRun(w.a1, "wake-1"), w.startRun(w.a2, "wake-2")
	holder, lease := map[string]*run{}, map[string]int64{}
	mk := w.sBen.mark()
	for round := 0; round < 6; round++ {
		w.advance(5000)
		tk := w.newTask(fmt.Sprintf("round-%d", round), "")
		var wg sync.WaitGroup
		for id, h := range holder {
			wg.Add(1)
			go func(id string, h *run) { // the holder renews while the other run tries to take its task
				defer wg.Done()
				w.runCall(h, "GET", w.podPath("me"), nil, nil)
				other := map[bool]*run{true: r2, false: r1}[h == r1]
				w.wantErr(w.claim(other, id), 409, "already_claimed", id+": a second run's claim on a held task")
			}(id, h)
		}
		var win *run
		wg.Add(1)
		go func() { defer wg.Done(); win, _ = w.raceClaim(tk.ID, r1, r2) }()
		w.heartbeat(w.compA)
		wg.Wait()
		holder[tk.ID] = win
		w.checkRound(holder, lease)
	}
	for id := range holder {
		if n := w.sBen.count(mk, func(f Frame) bool { var c Claim; f.dec(&c); return f.Type == "task.claimed" && c.TaskID == id }); n != 1 {
			t.Errorf("%s: %d task.claimed events, want exactly 1 (one holder, no re-claim)", id, n)
		}
		w.oneRelease(w.sBen, mk, id, 0)
	}
}

// checkRound waits until every task is held by its recorded run with the lease the ledger expects, and
// fails at once on a second holder or a lease that moved backward.
func (w *world) checkRound(holder map[string]*run, last map[string]int64) {
	w.t.Helper()
	w.eventually("renewal round", func() (bool, string) {
		all := true
		for _, tk := range w.tasks() {
			h, ok := holder[tk.ID]
			if !ok {
				continue
			}
			if tk.Claim != nil && tk.Claim.RunID != h.id {
				w.t.Fatalf("%s: held by %s, but %s won the race: two holders", tk.ID, tk.Claim.RunID, h.id)
			}
			if tk.Claim != nil && ms(tk.Claim.LeaseUntil) < last[tk.ID] {
				w.t.Fatalf("%s: leaseUntil moved backward: %s after %d", tk.ID, tk.Claim.LeaseUntil, last[tk.ID])
			}
			if okk, why := w.leaseOK(tk, h); !okk {
				all = false
				return false, why
			}
			last[tk.ID] = ms(tk.Claim.LeaseUntil)
		}
		return all, ""
	})
}

func (w *world) pause(d int64) {
	if realClock {
		time.Sleep(time.Duration(d) * time.Millisecond)
	} else {
		w.advance(d)
	}
}

// TestKillMidLease: kill -9 with two live claims. Y has no renewal after its claim and its deadline falls
// inside the outage; X was renewed by a heartbeat and its deadline falls after the restart. Neither may
// be released before its recorded deadline, and each must be released no later than
// max(restart, deadline) + JM_E2E_STALE_MS + 6 s once the clock passes it; no claim.released repeats.
func TestKillMidLease(t *testing.T) {
	need(t)
	if realClock {
		t.Parallel()
	}
	w := newWorld(t)
	stale := staleMs(t)
	ry, rx := w.startRun(w.a1, "wake-y"), w.startRun(w.b1, "wake-x")
	ty, tx := w.newTask("claim-y", ""), w.newTask("claim-x", "")
	w.mustClaim(ry, ty.ID)
	w.pause(15000)
	w.mustClaim(rx, tx.ID)
	w.pause(10000)
	w.heartbeat(w.compB)
	w.checkLease(tx.ID, rx)
	dy, dx := w.expected(ry), w.expected(rx)
	last := maxSeq(w.sBen.snapshot(0))
	w.oneRelease(w.sBen, 0, ty.ID, 0)
	w.oneRelease(w.sBen, 0, tx.ID, 0)
	healthy := w.crash(dy + 5000)
	s := w.openSince(w.ben.tok, "ben-after-restart", last)
	mk := s.mark()
	restartAt := healthy
	if !realClock {
		w.noRelease(s, ty.ID, 300*time.Millisecond)
		restartAt = dy + 10000
		w.setClock(restartAt)
	}
	w.dueAndReleased("after_restart_inside", s, mk, ty, ry, dy, max(restartAt, dy)+stale)
	if !realClock && w.clock < dx {
		if tk := w.task(tx.ID); tk.Claim == nil || tk.Claim.RunID != rx.id {
			t.Fatalf("x was released with the clock %d ms before its recorded deadline %d", dx-w.clock, dx)
		}
		w.setClock(dx - 1)
		w.noRelease(s, tx.ID, 300*time.Millisecond)
	}
	w.dueAndReleased("after_restart_ahead", s, mk, tx, rx, dx, max(restartAt, dx)+stale)
	if !realClock {
		w.advance(120000)
	}
	time.Sleep(300 * time.Millisecond)
	w.oneRelease(s, mk, ty.ID, 1)
	w.oneRelease(s, mk, tx.ID, 1)
	full := w.openSince(w.ben.tok, "ben-full-replay", 0)
	time.Sleep(500 * time.Millisecond)
	w.oneRelease(full, 0, ty.ID, 1)
	w.oneRelease(full, 0, tx.ID, 1)
}

// dueAndReleased moves the clock to due (fake) and waits for the expiry pair, then checks the bounds.
// In real-clock mode due is a wall-clock limit and it prints LATENESS_MS <kind> <ms>.
func (w *world) dueAndReleased(kind string, s *sock, mk int, tk Task, r *run, deadline, due int64) {
	w.t.Helper()
	wait := 6 * time.Second
	if realClock {
		wait = time.Until(time.UnixMilli(due)) + 6*time.Second
	} else if w.clock < due {
		w.setClock(due)
	}
	obs, ok := w.awaitExpiry(s, mk, tk.ID, r, wait)
	if !ok {
		w.t.FailNow()
	}
	if realClock {
		fmt.Printf("LATENESS_MS %s %d\n", kind, obs-deadline)
		if obs < deadline-2 || obs > due+6000 {
			w.t.Fatalf("%s released at %d: want within [%d, %d] (deadline, then max(restart, deadline)+stale+6 s)", tk.ID, obs, deadline, due+6000)
		}
	}
}
