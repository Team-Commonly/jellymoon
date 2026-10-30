package e2e

import (
	"testing"
	"time"
)

const quiet = 400 * time.Millisecond

// TestF07: only addressed changes wake an agent, wakes coalesce, unacked wakes are re-sent on the next
// heartbeat after 60 s, and run.wake goes only to the computer that hosts the agent.
func TestF07(t *testing.T) {
	needFake(t)
	w := newWorld(t)
	a, b, wake := w.compA.sock, w.compB.sock, isType("run.wake")
	mkA, mkB := a.mark(), b.mark()
	// Step 1: an ambient human message produces no run.wake.
	mkBen := w.sBen.mark()
	w.sO.send("message.post", map[string]any{"text": "morning all, nobody is addressed here"})
	w.expect(w.sBen, mkBen, isType("message.created"), "message.created")
	w.never(a, mkA, wake, quiet, "an ambient message produced a run.wake")
	// Step 2: three tasks assigned in a row make one pending wake, delivered once.
	t1 := w.newTask("first", w.a1.id)
	w.newTask("second", w.a1.id)
	w.newTask("third", w.a1.id)
	var wk Wake
	w.expect(a, mkA, wakeFor(w.a1.id), "run.wake after three assignments").dec(&wk)
	wantID(t, "wak_", wk.WakeID)
	if wk.Reason != "task_assigned" || wk.Refs.TaskID != t1.ID {
		t.Fatalf("wake reason=%q refs.taskId=%q, want task_assigned and the first task %s", wk.Reason, wk.Refs.TaskID, t1.ID)
	}
	after := a.mark()
	w.heartbeat(w.compA)
	w.never(a, after, wake, quiet, "the wake was delivered twice at the same clock")
	// Step 3: not acked within 60 s means re-sent on the next heartbeat (>= 60 000 ms, hosting computer only).
	c0 := w.clock
	w.setClock(c0 + leaseMs - 1)
	w.heartbeat(w.compA)
	w.never(a, after, wake, quiet, "wake re-sent 1 ms early")
	w.setClock(c0 + leaseMs)
	w.heartbeat(w.compB)
	w.never(a, after, wake, quiet, "a heartbeat from the other computer re-sent the wake")
	w.never(b, mkB, wake, 0, "a heartbeat from the other computer moved the wake")
	w.heartbeat(w.compA)
	var again Wake
	w.expect(a, after, wakeFor(w.a1.id), "re-sent wake at exactly 60 s").dec(&again)
	if again.WakeID != wk.WakeID {
		t.Fatalf("re-sent wake has id %s, want the same wake %s", again.WakeID, wk.WakeID)
	}
	// An ack changes nothing (ADR 0003): only a run claim consumes the wake, so an acked wake is re-sent like any other.
	a.send("wake.ack", map[string]any{"wakeId": wk.WakeID})
	time.Sleep(200 * time.Millisecond)
	after = a.mark()
	w.advance(2 * leaseMs)
	w.heartbeat(w.compA)
	var acked Wake
	w.expect(a, after, wakeFor(w.a1.id), "an acked wake is re-sent until a run claims it").dec(&acked)
	if acked.WakeID != wk.WakeID {
		t.Fatalf("the re-sent acked wake has id %s, want the same wake %s", acked.WakeID, wk.WakeID)
	}
	// Step 4: run.wake goes only to the computer that hosts the agent.
	mkA = a.mark()
	w.newTask("for-bee", w.b1.id)
	w.expect(b, mkB, wakeFor(w.b1.id), "run.wake for agent-bee on computer B")
	if a.count(mkA, wake) != 0 || b.count(mkB, wakeFor(w.a1.id)) != 0 {
		t.Fatal("a run.wake reached a computer that does not host the agent")
	}
}
