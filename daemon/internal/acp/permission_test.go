package acp

import (
	"context"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func spec(t *testing.T, mode string) (Spec, string) {
	rec := filepath.Join(t.TempDir(), "rec")
	return Spec{Command: []string{os.Args[0]}, Dir: t.TempDir(), Prompt: "go", Env: []string{"ACP_AGENT=" + mode, "ACP_REC=" + rec}}, rec
}

func events(rec string) []string {
	raw, _ := os.ReadFile(rec)
	return strings.Fields(string(raw))
}

// waitFor polls the agent's record until it holds an event.
func waitFor(t *testing.T, rec, event string) {
	t.Helper()
	for end := time.Now().Add(10 * time.Second); time.Now().Before(end); time.Sleep(10 * time.Millisecond) {
		if strings.Contains(strings.Join(events(rec), " "), event) {
			return
		}
	}
	t.Fatalf("the agent never recorded %q; it recorded %v", event, events(rec))
}

func answered(t *testing.T, s Spec, rec string) string {
	t.Helper()
	if err := Run(context.Background(), s); err != nil {
		t.Fatalf("Run = %v", err)
	}
	for _, e := range events(rec) {
		if a, ok := strings.CutPrefix(e, "answer:"); ok {
			return a
		}
	}
	t.Fatalf("the agent recorded no answer: %v", events(rec))
	return ""
}

// A permission request is rejected with the request's own reject option unless the agent was added with the allow setting.
func TestPermissionIsRejectedByDefault(t *testing.T) {
	for kinds, want := range map[string]string{
		"allow_once,allow_always,reject_always,reject_once": "reject_once",
		"allow_once,reject_always":                          "reject_always",
		"allow_once,allow_always":                           "cancelled", // no reject option is offered
	} {
		s, rec := spec(t, "ask:"+kinds)
		if got := answered(t, s, rec); got != want {
			t.Errorf("options %s: answered %s, want %s", kinds, got, want)
		}
	}
}

// With the allow setting the answer is allow_once, never allow_always, and a request with no allow_once is still rejected.
func TestPermissionAllowSettingGetsAllowOnce(t *testing.T) {
	for kinds, want := range map[string]string{
		"allow_always,allow_once,reject_once": "allow_once",
		"allow_always,reject_once":            "reject_once",
	} {
		s, rec := spec(t, "ask:"+kinds)
		s.Allow = true
		if got := answered(t, s, rec); got != want {
			t.Errorf("options %s: answered %s, want %s", kinds, got, want)
		}
	}
}

// Each request and its answer is logged by the call's id and kind, never by its title or input.
func TestPermissionIsLoggedWithoutArguments(t *testing.T) {
	s, rec := spec(t, "ask:allow_once,reject_once")
	var logs []string
	s.Logf = func(f string, a ...any) { logs = append(logs, fmt.Sprintf(f, a...)) }
	answered(t, s, rec)
	got := strings.Join(logs, "\n")
	if len(logs) != 1 || !strings.Contains(got, "call_1") || !strings.Contains(got, "reject_once") || strings.Contains(got, "SECRET_ARG") {
		t.Errorf("log = %q, want one line with the call id and the answer and no title", got)
	}
}

// Any other request from the agent still gets -32601.
func TestOtherAgentRequestsStayUnsupported(t *testing.T) {
	s, rec := spec(t, "ask-other")
	if err := Run(context.Background(), s); err != nil {
		t.Fatal(err)
	}
	if got := strings.Join(events(rec), " "); !strings.Contains(got, "other-error:-32601") {
		t.Errorf("record = %v, want the -32601 error", events(rec))
	}
}

var errStopped = errors.New("stopped")

// A run stopped mid-turn gets session/cancel before any signal; an agent that ends its turn on it leaves without being signalled.
func TestStopSendsCancelBeforeAnySignal(t *testing.T) {
	s, rec := spec(t, "cancel")
	ctx, cancel := context.WithCancelCause(context.Background())
	done := make(chan error, 1)
	go func() { done <- Run(ctx, s) }()
	waitFor(t, rec, "prompt")
	cancel(errStopped)
	select {
	case err := <-done:
		if !errors.Is(err, errStopped) {
			t.Errorf("Run = %v, want the stop's cause", err)
		}
	case <-time.After(10 * time.Second):
		t.Fatal("Run did not return after the stop")
	}
	if got := strings.Join(events(rec), " "); got != "prompt cancel" {
		t.Errorf("agent events = %q, want \"prompt cancel\": cancelled, and never signalled", got)
	}
}

// An agent that ignores session/cancel is signalled only after the wait.
func TestAgentThatIgnoresCancelIsSignalledAfterTheWait(t *testing.T) {
	s, rec := spec(t, "deaf")
	ctx, cancel := context.WithCancelCause(context.Background())
	done := make(chan error, 1)
	go func() { done <- Run(ctx, s) }()
	waitFor(t, rec, "prompt")
	began := time.Now()
	cancel(errStopped)
	if err := <-done; !errors.Is(err, errStopped) {
		t.Errorf("Run = %v, want the stop's cause", err)
	}
	if took := time.Since(began); took < 1900*time.Millisecond {
		t.Errorf("Run returned %v after the stop; the agent got less than the 2 s wait", took)
	}
	if got := strings.Join(events(rec), " "); got != "prompt cancel term" {
		t.Errorf("agent events = %q, want \"prompt cancel term\"", got)
	}
}
