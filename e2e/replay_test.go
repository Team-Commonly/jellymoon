package e2e

import (
	"testing"
	"time"
)

// seqs returns the sequence numbers of the sequenced frames at or below upTo, in arrival order.
func seqs(fs []Frame, upTo int64) (out []int64) {
	for _, f := range fs {
		if f.Seq > 0 && f.Seq <= upTo {
			out = append(out, f.Seq)
		}
	}
	return
}

// TestReplayAcrossRestart: after kill -9 and a restart, subscribe sinceSeq=N replays every event after
// N in order, with no gaps or duplicates. New events then continue after the old head.
func TestReplayAcrossRestart(t *testing.T) {
	needFake(t)
	w := newWorld(t)
	r := w.startRun(w.a1, "wake")
	for _, text := range []string{"one", "two", "three"} {
		w.sO.send("message.post", map[string]any{"text": text})
	}
	tk := w.newTask("replayed", "")
	w.mustClaim(r, tk.ID)
	w.must(w.c.do("POST", w.podPath("tasks", tk.ID, "release"), w.ben.tok, map[string]any{}, nil), "release")
	w.expect(w.sBen, 0, released(tk.ID, "released"), "claim.released before the kill")
	time.Sleep(200 * time.Millisecond)
	live := w.sBen.snapshot(0)
	head := maxSeq(live)
	if len(seqs(live, head)) < 6 {
		t.Fatalf("setup produced only %d sequenced events", len(seqs(live, head)))
	}
	n0 := seqs(live, head)[2]
	w.crash(0)
	w.setClock(w.clock)
	for _, from := range []int64{n0, 0} {
		s := w.openSince(w.ben.tok, "replay", from)
		var hello struct{ HeadSeq int64 }
		w.expect(s, 0, isType("hello"), "hello").dec(&hello)
		if hello.HeadSeq < head {
			t.Fatalf("after restart hello.headSeq=%d, want at least %d: events were lost", hello.HeadSeq, head)
		}
		if hello.HeadSeq > from {
			w.expect(s, 0, func(f Frame) bool { return f.Seq == hello.HeadSeq }, "replay up to the head")
		}
		time.Sleep(200 * time.Millisecond)
		got := seqs(s.snapshot(0), hello.HeadSeq)
		if int64(len(got)) != hello.HeadSeq-from {
			t.Fatalf("sinceSeq=%d: replayed %v, want %d consecutive events up to %d", from, got, hello.HeadSeq-from, hello.HeadSeq)
		}
		for i, q := range got {
			if q != from+int64(i)+1 {
				t.Fatalf("sinceSeq=%d: replay %v has a gap, duplicate or disorder at position %d", from, got, i)
			}
		}
	}
	w.sO = w.open(w.owner.tok, "olivia-again")
	mk := w.sO.mark()
	w.sO.send("message.post", map[string]any{"text": "after the restart"})
	if f := w.expect(w.sO, mk, isType("message.created"), "message.created after the restart"); f.Seq <= head {
		t.Fatalf("first event after the restart has seq %d, want greater than the pre-kill head %d", f.Seq, head)
	}
}
