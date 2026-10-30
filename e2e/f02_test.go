package e2e

import (
	"bytes"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"sync/atomic"
	"testing"
	"time"
)

// Steps counted across the run, for the closing line. Only a stepper that a real test folds in with count() adds to them.
var stepsRun, stepsOK atomic.Int64

const eventTail = 20

// stepper runs named steps. A failing step names itself and prints the last eventTail events the suite saw.
type stepper struct {
	out    io.Writer
	events func() []Frame
	run    int
	ok     int
}

func (p *stepper) step(name string, fn func() error) bool {
	p.run++
	err := fn()
	if err == nil {
		p.ok++
		return true
	}
	fmt.Fprintf(p.out, "STEP FAILED: %s: %v\n", name, err)
	evs := p.events()
	evs = evs[max(0, len(evs)-eventTail):]
	fmt.Fprintf(p.out, "last %d events:\n", len(evs))
	for _, f := range evs {
		d := string(f.Data)
		if len(d) > 120 {
			d = d[:120] + "..."
		}
		fmt.Fprintf(p.out, "  seq=%d type=%s data=%s\n", f.Seq, f.Type, d)
	}
	return false
}

func (p *stepper) count() {
	stepsRun.Add(int64(p.run))
	stepsOK.Add(int64(p.ok))
}

func summaryLine(ms, ok, run int64) string { return fmt.Sprintf("E2E_MS=%d STEPS=%d/%d", ms, ok, run) }

// printerCheck drives the printer with a fake step that fails, over 25 events, and judges what it wrote.
func printerCheck() error {
	var evs []Frame
	for i := int64(1); i <= 25; i++ {
		evs = append(evs, Frame{Type: "message.created", Seq: i, Data: []byte(fmt.Sprintf(`{"n":%d}`, i))})
	}
	var buf bytes.Buffer
	p := &stepper{out: &buf, events: func() []Frame { return evs }}
	if p.step("the fake step", func() error { return fmt.Errorf("nothing happened") }) || p.run != 1 || p.ok != 0 {
		return fmt.Errorf("a failing step must report failure and count as run, not ok (run=%d ok=%d)", p.run, p.ok)
	}
	got := buf.String()
	if !strings.Contains(got, "STEP FAILED: the fake step") || !strings.Contains(got, "nothing happened") {
		return fmt.Errorf("the failing step did not name itself and its error:\n%s", got)
	}
	if n := strings.Count(got, "  seq="); n != eventTail {
		return fmt.Errorf("printed %d events, want the last %d:\n%s", n, eventTail, got)
	}
	if !strings.Contains(got, "seq=25 ") || !strings.Contains(got, "seq=6 ") || strings.Contains(got, "seq=5 ") {
		return fmt.Errorf("the events printed are not the last %d (seq 6 to 25):\n%s", eventTail, got)
	}
	return nil
}

// TestStepPrinter needs no server: a fake failing step prints its name and the last 20 events, and the suite stays green.
func TestStepPrinter(t *testing.T) {
	if err := printerCheck(); err != nil {
		t.Fatal(err)
	}
}

// TestF02: the harness boots the Worker with a persisted state dir, waits for /healthz, and reports as the closing line.
func TestF02(t *testing.T) {
	needFake(t)
	w := newWorld(t)
	p := &stepper{out: os.Stdout, events: func() []Frame { return w.sO.snapshot(0) }}
	p.step("the Worker starts with --persist-to a temp dir", func() error {
		state := filepath.Join(w.srv.dir, "state")
		if !strings.HasPrefix(w.srv.dir, os.TempDir()) {
			return fmt.Errorf("data dir %s is not under the temp dir %s", w.srv.dir, os.TempDir())
		}
		ents, err := os.ReadDir(state)
		if err != nil || len(ents) == 0 {
			return fmt.Errorf("%s holds no persisted state (err=%v)", state, err)
		}
		return nil
	})
	p.step("GET /healthz returns 200 within 10 s", func() error {
		if w.srv.bootTook > 10*time.Second {
			return fmt.Errorf("the first 200 took %v", w.srv.bootTook)
		}
		resp, err := http.Get(w.srv.base() + "/healthz")
		if err != nil {
			return err
		}
		resp.Body.Close()
		if resp.StatusCode != 200 {
			return fmt.Errorf("/healthz answered %d", resp.StatusCode)
		}
		return nil
	})
	p.step("a failing step names itself and prints the last 20 events", printerCheck)
	p.step("the run ends with E2E_MS=<n> STEPS=<k>/<k>", func() error {
		line := summaryLine(1234, 4, 4)
		if m := regexp.MustCompile(`^E2E_MS=\d+ STEPS=(\d+)/(\d+)$`).FindStringSubmatch(line); m == nil || m[1] != m[2] {
			return fmt.Errorf("closing line is %q", line)
		}
		return nil
	})
	p.count()
	if p.ok != p.run {
		t.Fatalf("%d of %d steps passed", p.ok, p.run)
	}
}
