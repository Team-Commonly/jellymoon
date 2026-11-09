package e2e

import (
	"net/http"
	"strings"
	"testing"
)

// A step that panics, as one does on a nil client left by a failed sign-in step, is a failed step: the steps after it still
// run and are counted, and the test reports every failure instead of dying on the first panic.
func TestStepperTurnsAPanicIntoAFailedStep(t *testing.T) {
	var out strings.Builder
	p := &stepper{out: &out, events: func() []Frame { return nil }}
	var hc *http.Client
	if p.step("uses a client a failed step left nil", func() error { _, err := hc.Get("http://127.0.0.1:1/"); return err }) {
		t.Fatal("a panicking step passed")
	}
	if !p.step("a later step", func() error { return nil }) {
		t.Fatal("the step after a panic did not run")
	}
	if p.run != 2 || p.ok != 1 {
		t.Fatalf("run=%d ok=%d, want 2 and 1", p.run, p.ok)
	}
	if !strings.Contains(out.String(), "STEP FAILED: uses a client a failed step left nil: panic:") {
		t.Fatalf("the failure line does not name the panic:\n%s", out.String())
	}
}
