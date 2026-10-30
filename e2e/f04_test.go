package e2e

import (
	"fmt"
	"os"
	"slices"
	"testing"
	"time"
)

// posted is the summary of every message.created in fs, with its seq, in arrival order.
func posted(fs []Frame) (texts []string, ss []int64) {
	for _, f := range fs {
		if f.Type != "message.created" {
			continue
		}
		var m struct{ Summary string }
		f.dec(&m)
		texts, ss = append(texts, m.Summary), append(ss, f.Seq)
	}
	return
}

// replayed reconnects as tok from sinceSeq and returns what the pod sends, once it has been quiet.
func (w *world) replayed(tok, name string, since int64) []Frame {
	s := w.openSince(tok, name, since)
	if _, ok := s.waitAfter(0, isType("hello"), 5*time.Second); !ok {
		w.t.Fatalf("%s: no hello within 5 s of the upgrade", name)
	}
	time.Sleep(quiet)
	return s.snapshot(0)
}

// wantReplay judges a replay: exactly the texts, in order, with consecutive seqs starting after `since`.
func wantReplay(fs []Frame, since int64, texts ...string) error {
	got, ss := posted(fs)
	if !slices.Equal(got, texts) {
		return fmt.Errorf("replayed %q, want exactly %q", got, texts)
	}
	for i, q := range ss {
		if q != since+int64(i)+1 {
			return fmt.Errorf("replayed seqs %v, want %d consecutive from %d", ss, len(texts), since+1)
		}
	}
	if n := len(seqs(fs, 1<<60)); n != len(texts) {
		return fmt.Errorf("replay carried %d sequenced events, want only the %d messages", n, len(texts))
	}
	return nil
}

// postAll has s post each text in turn and waits for each to be accepted.
func postAll(s *sock, texts ...string) error {
	for _, text := range texts {
		mk := s.mark()
		s.send("message.post", map[string]any{"text": text})
		if _, ok := s.waitAfter(mk, isType("message.created"), 5*time.Second); !ok {
			return fmt.Errorf("%q was not accepted", text)
		}
	}
	return nil
}

// TestF04: live fan-out, then a reconnect from sinceSeq with no gaps, then the same across a full restart.
func TestF04(t *testing.T) {
	needFake(t)
	w := newWorld(t)
	p := &stepper{out: os.Stdout, events: func() []Frame { return w.sCara.snapshot(0) }}
	var first int64
	p.step("two members subscribe; one sends message.post", func() error { return postAll(w.sBen, "live one") })
	p.step("both receive message.created with the same seq", func() error {
		var got [2]int64
		for i, s := range []*sock{w.sBen, w.sCara} {
			f, ok := s.waitAfter(0, isType("message.created"), 5*time.Second)
			if !ok {
				return fmt.Errorf("%s saw no message.created", s.name)
			}
			got[i] = f.Seq
		}
		if got[0] != got[1] || got[0] == 0 {
			return fmt.Errorf("seqs differ or are zero: ben %d, cara %d", got[0], got[1])
		}
		first = got[0]
		return nil
	})
	p.step("the second client disconnects, three more posts happen, it reconnects with subscribe{sinceSeq}", func() error {
		w.sCara.close()
		err := postAll(w.sBen, "missed one", "missed two", "missed three")
		w.sCara = w.dial(w.cara.tok, "cara-back")
		w.sCara.send("subscribe", map[string]any{"sinceSeq": first})
		time.Sleep(quiet)
		return err
	})
	p.step("it receives exactly the three missed events, in order", func() error {
		return wantReplay(w.sCara.snapshot(0), first, "missed one", "missed two", "missed three")
	})
	// A Durable Object cannot be evicted from outside under `wrangler dev`, so the black-box substitute is a full
	// restart (kill -9 of the server, same state dir) in place of hibernation. Hibernation itself is not asserted.
	p.step("the same holds after a full restart of the Worker (substituting for Durable Object hibernation)", func() error {
		w.sCara.close()
		if err := postAll(w.sBen, "after one", "after two", "after three"); err != nil {
			return err
		}
		w.crash(0)
		w.setClock(w.clock)
		return wantReplay(w.replayed(w.cara.tok, "cara-restart", first+3), first+3, "after one", "after two", "after three")
	})
	p.count()
	if p.ok != p.run {
		t.Fatalf("%d of %d steps passed", p.ok, p.run)
	}
}
