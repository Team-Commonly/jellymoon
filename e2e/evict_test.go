package e2e

import (
	"strings"
	"testing"
	"time"
)

type instance struct {
	Instance string
	Requests int
	Was      int64 // woke only: the stored alarm time the discard moved, which the runtime then fired
}

// discard has the test entry throw the pod's object away: the runtime ends its requests, drops every socket and builds a new
// instance for the next request or alarm. `now` is stored as the server clock with no tick; alarmMs > 0 moves the alarm the object
// itself stored to fire that many real ms ahead (it plants no new one, and refuses if there is none), and the clock becomes alarmNow
// when it runs. It returns the stored alarm time the object had at that moment. The suite sets w.clock to match, since the
// server clock moved without a /__test/clock call.
func (w *world) discard(now, alarmMs, alarmNow int64) int64 {
	w.t.Helper()
	body := map[string]any{"now": now}
	if alarmMs > 0 {
		body["alarmMs"], body["alarmNow"] = alarmMs, alarmNow
		now = alarmNow
	}
	var out struct{ Alarm *int64 }
	w.must(w.c.do("POST", "/__test/discard/"+w.pod, "", body, &out), "POST /__test/discard/:pod")
	w.clock = now
	if out.Alarm == nil {
		return 0
	}
	return *out.Alarm
}

// storedAlarm is the alarm time the object itself has stored (ctx.storage.getAlarm), read without counting as a request. Under the
// fake clock the object's deadline is a 2040 time, so it equals the deadline in unix ms and the runtime never fires it by itself.
func (w *world) storedAlarm() int64 {
	w.t.Helper()
	var a struct{ At *int64 }
	w.must(w.c.do("GET", "/__test/stored-alarm/"+w.pod, "", nil, &a), "GET /__test/stored-alarm/:pod")
	if a.At == nil {
		w.t.Fatalf("the object has no stored alarm while a claim is pending")
	}
	return *a.At
}

// wantAlarm fails unless the object's own stored alarm is the deadline the suite computed from its own ledger.
func (w *world) wantAlarm(want int64, when string) {
	w.t.Helper()
	if got := w.storedAlarm(); got != want {
		w.t.Fatalf("stored alarm %s is %d, want the claim's deadline %d", when, got, want)
	}
}

// instanceOf reads the object's instance id and the count of requests it has served (the read itself is not counted).
// If the object was discarded, the read is the request that builds the new one.
func (w *world) instanceOf() instance {
	w.t.Helper()
	var in instance
	w.must(w.c.do("GET", "/__test/instance/"+w.pod, "", nil, &in), "GET /__test/instance/:pod")
	return in
}

// reconnect opens fresh live sockets for everyone whose socket the discard dropped.
func (w *world) reconnect() {
	w.sO, w.sCara = w.open(w.owner.tok, "olivia-2"), w.open(w.cara.tok, "cara-2")
	w.compA.sock, w.compB.sock = w.open(w.compA.tok, "computer-a-2"), w.open(w.compB.tok, "computer-b-2")
}

// TestEvictMidClaim: the runtime discards the pod's object while a claim is live and the worker process stays up. The new
// instance restores the lease, a clock one second short of the deadline releases nothing, a renewal moves leaseUntil as
// ADR 0002 says, and a second run's claim is still refused with the holder named.
func TestEvictMidClaim(t *testing.T) {
	needFake(t)
	w := newWorld(t)
	r := w.startRun(w.a1, "wake")
	tk := w.newTask("evicted", "")
	w.mustClaim(r, tk.ID)
	w.checkLease(tk.ID, r)
	deadline := w.expected(r)
	w.wantAlarm(deadline, "after the claim") // the object stored its own deadline alarm
	last := maxSeq(w.sBen.snapshot(0))
	before := w.instanceOf()
	w.discard(deadline-1000, 0, 0)
	w.eventually("the discard dropping every socket", func() (bool, string) {
		for _, s := range []*sock{w.sO, w.sBen, w.sCara, w.compA.sock, w.compB.sock} {
			if !s.isDead() {
				return false, s.name + " is still open"
			}
		}
		return true, ""
	})
	s := w.openSince(w.ben.tok, "ben-after-discard", last) // the first request the new object serves
	w.reconnect()
	if after := w.instanceOf(); after.Instance == before.Instance {
		t.Fatalf("the object is the same instance %s after the discard: it was reset, not thrown away", after.Instance)
	}
	w.wantAlarm(deadline, "on the restored object") // the restore stored the pending deadline again
	w.noRelease(s, 0, tk.ID, 300*time.Millisecond)
	w.checkLease(tk.ID, r) // the lease survived with its old deadline
	w.runCall(r, "GET", w.podPath("me"), nil, nil)
	w.checkLease(tk.ID, r) // the renewal after the eviction moved it to clock + 60 s
	if w.expected(r) != deadline+59000 {
		t.Fatalf("renewal stamped %d, want %d", w.expected(r), deadline+59000)
	}
	w.wantAlarm(w.expected(r), "after the renewal") // the new object moved its alarm with the lease
	r2 := w.startRun(w.a2, "wake-2")
	rs := w.claim(r2, tk.ID)
	w.wantErr(rs, 409, "already_claimed", "a second run's claim after the eviction")
	if holder := rs.Err.Ref + " " + rs.Err.Message; !strings.Contains(holder, r.id) && !strings.Contains(holder, w.a1.id) && !strings.Contains(holder, w.a1.actor.DisplayName) {
		t.Errorf("already_claimed does not name the holder (run %s, agent %s): ref=%q message=%q", r.id, w.a1.id, rs.Err.Ref, rs.Err.Message)
	}
	w.setClock(deadline) // the old deadline passes: the renewal kept the claim
	w.noRelease(s, 0, tk.ID, 300*time.Millisecond)
	w.setClock(w.expected(r))
	if _, ok := w.awaitExpiry(s, 0, tk.ID, r, 5*time.Second); !ok {
		t.FailNow()
	}
	time.Sleep(300 * time.Millisecond)
	w.oneRelease(s, 0, tk.ID, 1)
}

// TestEvictAlarmWake: the object stores its own deadline alarm (read back, and equal to the deadline). It is discarded with the
// clock stored one ms short of the deadline, and that stored alarm is moved to fire a few real ms ahead. Nothing is sent to the
// object. The runtime builds a new instance and calls its alarm, which moves the clock to the deadline and ticks; members then see
// exactly one claim.released, stamped at the deadline or later. The object records which instance the alarm woke, how many requests
// it had served by then, and the time the alarm had before it was moved.
func TestEvictAlarmWake(t *testing.T) {
	needFake(t)
	w := newWorld(t)
	r := w.startRun(w.a1, "wake")
	tk := w.newTask("alarm", "")
	w.mustClaim(r, tk.ID)
	w.checkLease(tk.ID, r)
	deadline := w.expected(r)
	w.wantAlarm(deadline, "before the discard") // the object stored its own deadline alarm
	last := maxSeq(w.sBen.snapshot(0))
	mk := w.sBen.mark()
	w.setClock(deadline - 1)
	w.noRelease(w.sBen, mk, tk.ID, 300*time.Millisecond)
	before := w.instanceOf()
	if before.Requests == 0 {
		t.Fatalf("the request counter reads 0 on an object that has served the whole setup: it counts nothing")
	}
	if at := w.alarmFired(); at != nil {
		t.Fatalf("the alarm had fired (%s) before the eviction", *at)
	}
	if was := w.discard(deadline-1, 100, deadline); was != deadline { // moves the alarm the object stored; no request reaches it from here until the release is seen
		t.Fatalf("the discard moved a stored alarm at %d, want the claim's deadline %d", was, deadline)
	}
	woke := w.awaitWoke()
	if woke.Was != deadline {
		t.Fatalf("the alarm that ran was moved from %d, want the object's own deadline alarm %d", woke.Was, deadline)
	}
	if woke.Instance == before.Instance || woke.Requests != 0 {
		t.Fatalf("alarm ran on instance %s after %d requests; the discarded one was %s: want a new instance and no request", woke.Instance, woke.Requests, before.Instance)
	}
	members := w.membersReplay(tk, r, last, deadline)
	if now := w.instanceOf(); now.Instance != woke.Instance {
		t.Fatalf("the object was rebuilt again (%s, then %s): the alarm did not run on the instance that served the release", woke.Instance, now.Instance)
	}
	fired := w.alarmFired()
	if fired == nil {
		t.Fatalf("alarm_fired did not move: the release did not come from the alarm")
	}
	// Nothing re-arms the consumed alarm: alarm_fired holds still across two reads, and no repeat release follows a clock move.
	time.Sleep(500 * time.Millisecond)
	if again := w.alarmFired(); again == nil || *again != *fired {
		t.Fatalf("alarm_fired moved from %v to %v with nothing to wake the object: the alarm is firing again", *fired, again)
	}
	w.advance(120000)
	time.Sleep(300 * time.Millisecond)
	for _, m := range members {
		w.oneRelease(m, 0, tk.ID, 1)
	}
}

// alarmFired reads the object's alarm_fired marker: when its alarm handler last ran, or nil. The read is a request to the object.
func (w *world) alarmFired() *string {
	w.t.Helper()
	var fired struct{ At *string }
	w.must(w.c.do("POST", "/__test/alarm/"+w.pod, "", nil, &fired), "POST /__test/alarm/:pod")
	return fired.At
}

// awaitWoke waits for the alarm handler's record in D1, which is read without sending the object a request.
func (w *world) awaitWoke() instance {
	w.t.Helper()
	var woke instance
	w.eventually("the alarm waking the discarded object", func() (bool, string) {
		woke = instance{}
		w.must(w.c.do("GET", "/__test/woke/"+w.pod, "", nil, &woke), "GET /__test/woke/:pod")
		return woke.Instance != "", "no record from the alarm yet"
	})
	return woke
}

// membersReplay opens a fresh replay from `since` for Ben, Olivia and Cara, waits for the expiry on each, checks the release is
// stamped at the deadline or later (the alarm's tick, not the restore before it), and that each carries exactly one.
func (w *world) membersReplay(tk Task, r *run, since, deadline int64) []*sock {
	w.t.Helper()
	members := []*sock{w.openSince(w.ben.tok, "ben-after-alarm", since), w.openSince(w.owner.tok, "olivia-after-alarm", since), w.openSince(w.cara.tok, "cara-after-alarm", since)}
	for _, m := range members {
		if _, ok := w.awaitExpiry(m, 0, tk.ID, r, 5*time.Second); !ok {
			w.t.FailNow()
		}
		if f, _ := m.waitAfter(0, released(tk.ID, "expired"), time.Second); ms(f.Ts) < deadline {
			w.t.Errorf("%s: claim.released is stamped %d, before the deadline %d: it came from the restore, not the alarm's tick", m.name, ms(f.Ts), deadline)
		}
	}
	time.Sleep(300 * time.Millisecond)
	for _, m := range members {
		w.oneRelease(m, 0, tk.ID, 1) // every member's replay carries exactly one
	}
	return members
}

// TestEvictPastDeadline: the object is discarded with the clock stored past the deadline, and the first thing to reach the new
// instance is a socket reconnect, which runs no command. The release must come from the restore itself (`boot`), once.
func TestEvictPastDeadline(t *testing.T) {
	needFake(t)
	w := newWorld(t)
	r := w.startRun(w.a1, "wake")
	tk := w.newTask("past due", "")
	w.mustClaim(r, tk.ID)
	w.checkLease(tk.ID, r)
	last := maxSeq(w.sBen.snapshot(0))
	w.discard(w.expected(r)+5000, 0, 0)
	s := w.openSince(w.ben.tok, "ben-after-discard", last)
	if _, ok := w.awaitExpiry(s, 0, tk.ID, r, 5*time.Second); !ok {
		t.FailNow()
	}
	time.Sleep(300 * time.Millisecond)
	w.oneRelease(s, 0, tk.ID, 1)
}
