package acp

import (
	"context"
	"strings"
	"testing"
)

// Only end_turn is a finished turn. A refusal, or a turn the agent had to cut short, is an error, so the daemon reports the run
// failed and keeps its workspace; the error names the stop reason.
func TestStopReasonDecidesTheTurn(t *testing.T) {
	for _, c := range []struct {
		reason  string
		wantErr bool
	}{
		{"end_turn", false},
		{"refusal", true},
		{"max_tokens", true},
		{"max_turn_requests", true},
		{"cancelled", false}, // a stop the client asked for is reported by Run through its cause, not here
		{"none", false},      // no stopReason, or one this client does not know: the run is left as completed
		{"something_new", false},
	} {
		s, _ := spec(t, "stop:"+c.reason)
		err := Run(context.Background(), s)
		if c.wantErr && (err == nil || !strings.Contains(err.Error(), c.reason)) {
			t.Errorf("stopReason %s: Run = %v, want an error naming it", c.reason, err)
		}
		if !c.wantErr && err != nil {
			t.Errorf("stopReason %s: Run = %v, want nil", c.reason, err)
		}
	}
}
