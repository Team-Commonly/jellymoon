package e2e

import "time"

func released(taskID, cause string) func(Frame) bool {
	return func(f Frame) bool {
		var r Released
		f.dec(&r)
		return f.Type == "claim.released" && r.TaskID == taskID && (cause == "" || r.Cause == cause)
	}
}

func unclaimed(taskID string) func(Frame) bool {
	return func(f Frame) bool {
		var tk Task
		f.dec(&tk)
		return f.Type == "task.updated" && tk.ID == taskID && tk.Claim == nil
	}
}

func maxSeq(fs []Frame) int64 {
	var m int64
	for _, f := range fs {
		if f.Seq > m {
			m = f.Seq
		}
	}
	return m
}

// awaitExpiry waits up to wait for the pair an expiry must send, task.updated with no claim and
// claim.released{expired} with no by, and returns the earlier receive time in unix ms.
func (w *world) awaitExpiry(s *sock, mk int, taskID string, r *run, wait time.Duration) (int64, bool) {
	w.t.Helper()
	end := time.Now().Add(wait)
	f1, ok1 := s.waitAfter(mk, unclaimed(taskID), wait)
	f2, ok2 := s.waitAfter(mk, released(taskID, "expired"), time.Until(end))
	if !ok1 || !ok2 {
		w.t.Errorf("%s: expiry of %s within %v: task.updated(no claim) seen=%v, claim.released{expired} seen=%v", s.name, taskID, wait, ok1, ok2)
		return 0, false
	}
	var rel Released
	f2.dec(&rel)
	if rel.RunID != r.id || rel.By != nil {
		w.t.Errorf("claim.released{expired}: runId=%q by=%v, want runId=%s and no by", rel.RunID, rel.By, r.id)
	}
	return min(f1.recv, f2.recv), true
}

// noRelease fails when a claim.released for taskID is on s after mk, or arrives within d. Take mk BEFORE the step under test
// (a clock move): a server that releases early sends claim.released before /__test/clock returns, and a mark taken after
// the move would start past it.
func (w *world) noRelease(s *sock, mk int, taskID string, d time.Duration) {
	w.t.Helper()
	if !s.none(mk, released(taskID, ""), d) {
		w.t.Fatalf("%s: claim.released for %s arrived before its deadline", s.name, taskID)
	}
}

// oneRelease checks that s saw exactly n claim.released for taskID since mk.
func (w *world) oneRelease(s *sock, mk int, taskID string, n int) {
	w.t.Helper()
	if got := s.count(mk, released(taskID, "")); got != n {
		w.t.Errorf("%s: %d claim.released for %s, want exactly %d", s.name, got, taskID, n)
	}
}

// expect waits up to 5 s for a frame and fails the test with what if none comes.
func (w *world) expect(s *sock, mk int, pred func(Frame) bool, what string) Frame {
	w.t.Helper()
	f, ok := s.waitAfter(mk, pred, 5*time.Second)
	if !ok {
		w.t.Fatalf("%s: no %s within 5 s", s.name, what)
	}
	return f
}

// never fails the test if a matching frame arrives within d.
func (w *world) never(s *sock, mk int, pred func(Frame) bool, d time.Duration, what string) {
	w.t.Helper()
	if !s.none(mk, pred, d) {
		w.t.Fatalf("%s: %s", s.name, what)
	}
}
