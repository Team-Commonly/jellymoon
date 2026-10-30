package e2e

import (
	"strings"
	"testing"
	"time"
)

type instance struct {
	Instance string
	Requests int
}

// discard has the test entry throw the pod's object away: the runtime ends its requests, drops every socket and builds a new
// instance for the next request or alarm. `now` is stored as the server clock with no tick; alarmMs > 0 stores an alarm that
// many real ms ahead. The suite sets w.clock to match, since the server clock moved without a /__test/clock call.
func (w *world) discard(now, alarmMs int64) {
	w.t.Helper()
	body := map[string]any{"now": now}
	if alarmMs > 0 {
		body["alarmMs"] = alarmMs
	}
	w.must(w.c.do("POST", "/__test/discard/"+w.pod, "", body, nil), "POST /__test/discard/:pod")
	w.clock = now
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
	last := maxSeq(w.sBen.snapshot(0))
	before := w.instanceOf()
	w.discard(deadline-1000, 0)
	s := w.openSince(w.ben.tok, "ben-after-discard", last) // the first request the new object serves
	w.reconnect()
	if after := w.instanceOf(); after.Instance == before.Instance {
		t.Fatalf("the object is the same instance %s after the discard: it was reset, not thrown away", after.Instance)
	}
	w.noRelease(s, 0, tk.ID, 300*time.Millisecond)
	w.checkLease(tk.ID, r) // the lease survived with its old deadline
	w.runCall(r, "GET", w.podPath("me"), nil, nil)
	w.checkLease(tk.ID, r) // the renewal after the eviction moved it to clock + 60 s
	if w.expected(r) != deadline+59000 {
		t.Fatalf("renewal stamped %d, want %d", w.expected(r), deadline+59000)
	}
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

// TestEvictAlarmWake: the object is discarded with a claim live, the clock stored at the deadline, and an alarm stored a few
// real ms ahead. Nothing is sent to the object. The runtime builds a new instance and calls its alarm; members then see exactly
// one claim.released. The object itself records which instance the alarm woke and how many requests it had served by then.
func TestEvictAlarmWake(t *testing.T) {
	needFake(t)
	w := newWorld(t)
	r := w.startRun(w.a1, "wake")
	tk := w.newTask("alarm", "")
	w.mustClaim(r, tk.ID)
	w.checkLease(tk.ID, r)
	deadline := w.expected(r)
	last := maxSeq(w.sBen.snapshot(0))
	mk := w.sBen.mark()
	w.setClock(deadline - 1)
	w.noRelease(w.sBen, mk, tk.ID, 300*time.Millisecond)
	before := w.instanceOf()
	var fired struct{ At *string }
	w.must(w.c.do("POST", "/__test/alarm/"+w.pod, "", nil, &fired), "POST /__test/alarm/:pod")
	if fired.At != nil {
		t.Fatalf("the alarm had fired (%s) before the eviction", *fired.At)
	}
	w.discard(deadline, 100) // no request reaches the object from here until the release is seen
	var woke instance
	w.eventually("the alarm waking the discarded object", func() (bool, string) {
		woke = instance{}
		w.must(w.c.do("GET", "/__test/woke/"+w.pod, "", nil, &woke), "GET /__test/woke/:pod")
		return woke.Instance != "", "no record from the alarm yet"
	})
	if woke.Instance == before.Instance || woke.Requests != 0 {
		t.Fatalf("alarm ran on instance %s after %d requests; the discarded one was %s: want a new instance and no request", woke.Instance, woke.Requests, before.Instance)
	}
	s := w.openSince(w.ben.tok, "ben-after-alarm", last)
	if _, ok := w.awaitExpiry(s, 0, tk.ID, r, 5*time.Second); !ok {
		t.FailNow()
	}
	time.Sleep(300 * time.Millisecond)
	w.oneRelease(s, 0, tk.ID, 1)
	if now := w.instanceOf(); now.Instance != woke.Instance {
		t.Fatalf("the object was rebuilt again (%s, then %s): the alarm did not run on the instance that served the release", woke.Instance, now.Instance)
	}
	w.must(w.c.do("POST", "/__test/alarm/"+w.pod, "", nil, &fired), "POST /__test/alarm/:pod")
	if fired.At == nil {
		t.Fatalf("alarm_fired did not move: the release did not come from the alarm")
	}
	w.advance(120000)
	time.Sleep(300 * time.Millisecond)
	w.oneRelease(s, 0, tk.ID, 1)
}
