package daemon

import (
	"context"
	"fmt"
	"io"
	"log"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync/atomic"
	"testing"

	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

func TestCheckDisplayName(t *testing.T) {
	for _, ok := range []string{"walkbot", "Walk-Bot", "walk_bot.2", "claude-code", "a", "-lead"} {
		if err := CheckDisplayName(ok); err != nil {
			t.Errorf("%q refused: %v", ok, err)
		}
	}
	cases := map[string]string{
		"Walk Bot":  "Walk-Bot",
		"Ada's Bot": "Ada-s-Bot",
		"bot!":      "bot",
		"walkbot-":  "walkbot",
		"naïve":     "na-ve",
		"@bot":      "bot",
		"":          "",
		"  ":        "",
	}
	for bad, suggest := range cases {
		err := CheckDisplayName(bad)
		if err == nil {
			t.Errorf("%q accepted", bad)
			continue
		}
		if !strings.Contains(err.Error(), "letters, digits") {
			t.Errorf("%q: message does not name the allowed characters: %v", bad, err)
		}
		if suggest != "" && !strings.Contains(err.Error(), fmt.Sprintf("%q", suggest)) {
			t.Errorf("%q: message does not suggest %q: %v", bad, suggest, err)
		}
	}
}

func endedContext() context.Context {
	ctx, cancel := context.WithCancel(context.Background())
	cancel() // watch starts no session on an ended daemon
	return ctx
}

// A refused add never reaches the server, so no agent row is created.
func TestAddRefusesWhatCanNeverWorkBeforeRegistering(t *testing.T) {
	var hits atomic.Int64
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		hits.Add(1)
		_, _ = io.WriteString(w, `{"agentId":"agt_1"}`)
	}))
	defer srv.Close()
	c, err := api.New(srv.URL)
	if err != nil {
		t.Fatal(err)
	}
	store := config.Store{Dir: t.TempDir()}
	d := New(c, store, config.Computer{Token: "jmc_x"}, log.New(io.Discard, "", 0))
	d.ctx = endedContext()
	t.Setenv("JELLYMOON_HARNESS_CLAUDE_CODE", "")
	_, err = d.add(AddRequest{PodID: "pod_a", Harness: protocol.HarnessClaudeCode, DisplayName: "claude-code"})
	if err == nil || !strings.Contains(err.Error(), "no program is configured for harness claude-code; set JELLYMOON_HARNESS_CLAUDE_CODE") {
		t.Errorf("unconfigured harness: err = %v", err)
	}
	if _, err = d.add(AddRequest{PodID: "pod_a", Harness: protocol.HarnessFake, DisplayName: "Walk Bot"}); err == nil {
		t.Error("a name with a space was accepted")
	}
	if hits.Load() != 0 {
		t.Errorf("server was called %d times for refused adds", hits.Load())
	}
	if got, _ := store.Agents(); len(got) != 0 {
		t.Errorf("%d agents remembered after refused adds", len(got))
	}
	// fake needs no program at add time; a configured harness is accepted too.
	if _, err = d.add(AddRequest{PodID: "pod_a", Harness: protocol.HarnessFake, DisplayName: "fake-one"}); err != nil {
		t.Errorf("fake refused: %v", err)
	}
	t.Setenv("JELLYMOON_HARNESS_CLAUDE_CODE", "/usr/bin/true")
	if _, err = d.add(AddRequest{PodID: "pod_a", Harness: protocol.HarnessClaudeCode, DisplayName: "claude-code"}); err != nil {
		t.Errorf("configured harness refused: %v", err)
	}
}
