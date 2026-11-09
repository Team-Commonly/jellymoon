package acp

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"strings"
	"testing"
	"time"
)

var errDeadline = errors.New("deadline")

// stop ends the run's context with cause once the agent has the prompt, and returns Run's error and the agent's record.
func stop(t *testing.T, mode string, cause error) (string, error) {
	s, rec := spec(t, mode)
	ctx, cancel := context.WithCancelCause(context.Background())
	done := make(chan error, 1)
	go func() { done <- Run(ctx, s) }()
	waitFor(t, rec, "prompt")
	cancel(cause)
	select {
	case err := <-done:
		return rec, err
	case <-time.After(10 * time.Second):
		t.Fatalf("mode %s: Run did not return after the stop", mode)
		return rec, nil
	}
}

// However the session ends after a stop (stopReason cancelled, a JSON-RPC error, the agent leaving unanswered) Run reports the
// stop's cause; a deadline's cause is its own, so that run still fails.
func TestAnyEndingAfterAStopReportsTheStopsCause(t *testing.T) {
	for _, mode := range []string{"cancel", "error-on-cancel", "exit-on-cancel"} {
		for _, cause := range []error{errStopped, errDeadline} {
			if _, err := stop(t, mode, cause); !errors.Is(err, cause) {
				t.Errorf("mode %s: Run = %v, want %v", mode, err, cause)
			}
		}
	}
}

// What the client writes back for one permission request, through serve.
func TestPermissionAnswerShape(t *testing.T) {
	bad := `{"kind":"reject_once"},{"optionId":5,"kind":"reject_once"},{"optionId":"","kind":"allow_once"}`
	both := `{"optionId":"id-a","kind":"allow_once"},{"optionId":"id-r","kind":"reject_once"}`
	picked := func(id, opt string) string {
		return `"id":` + id + `,"jsonrpc":"2.0","result":{"outcome":{"optionId":"` + opt + `","outcome":"selected"}}`
	}
	none := `"result":{"outcome":{"outcome":"cancelled"}}`
	for i, c := range []struct {
		allow, cancelling bool
		id, options, want string
	}{
		{false, false, `7`, bad, none}, {true, false, `7`, bad, none}, // no usable optionId: never selected
		{false, false, `7`, `{"kind":"reject_once"},{"optionId":"two","kind":"reject_once"}`, picked(`7`, "two")},
		{true, true, `7`, both, none},                          // after session/cancel, even allowed
		{false, false, `"x-1"`, both, picked(`"x-1"`, "id-r")}, // the optionId, not the kind; the request's own id
		{true, false, `7`, both, picked(`7`, "id-a")},
	} {
		var out bytes.Buffer
		cn := &conn{in: &out, allow: c.allow, cancelling: c.cancelling}
		cn.serve(message{ID: json.RawMessage(c.id), Method: "session/request_permission", Params: json.RawMessage(`{"options":[` + c.options + `]}`)})
		if !strings.Contains(out.String(), c.want) {
			t.Errorf("case %d: answered %s, want %s", i, out.String(), c.want)
		}
	}
}

// An agent that ignores SIGTERM is killed after it: Run returns instead of waiting for the agent to leave on its own.
func TestAgentThatIgnoresSIGTERMIsKilled(t *testing.T) {
	began := time.Now()
	_, _ = stop(t, "stubborn", errStopped)
	if took := time.Since(began); took > 8*time.Second {
		t.Errorf("Run took %v; it waited for the agent instead of sending SIGKILL", took)
	}
}

// An agent that leaves with no stop asked is a failed run: only a stop turns the ending into the stop's cause.
func TestAgentThatLeavesWithNoStopFails(t *testing.T) {
	s, _ := spec(t, "exit-on-prompt")
	if err := Run(context.Background(), s); err == nil {
		t.Error("Run = nil, want the agent's own failure")
	}
}

// A stop in flight decides the ending: an agent that answers session/cancel with a refusal or a token limit instead of "cancelled"
// still ends the run for the stop's cause, so the daemon reports it cancelled, not failed. A deadline's cause stays its own.
func TestStopReasonDoesNotOutrankAStopInFlight(t *testing.T) {
	for _, reason := range []string{"refusal", "max_tokens", "max_turn_requests"} {
		for _, cause := range []error{errStopped, errDeadline} {
			if _, err := stop(t, "cancel:"+reason, cause); !errors.Is(err, cause) {
				t.Errorf("stopReason %s after a stop (%v): Run = %v, want %v", reason, cause, err, cause)
			}
		}
	}
}
