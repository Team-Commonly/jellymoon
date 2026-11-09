package cli

import (
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync"
	"testing"

	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
)

// recorder answers every request 401 and keeps the Authorization headers it saw.
func recorder(t *testing.T) (*httptest.Server, func() []string) {
	var mu sync.Mutex
	var seen []string
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		mu.Lock()
		seen = append(seen, r.Header.Get("Authorization"))
		mu.Unlock()
		w.WriteHeader(http.StatusUnauthorized)
	}))
	t.Cleanup(srv.Close)
	return srv, func() []string { mu.Lock(); defer mu.Unlock(); return append([]string(nil), seen...) }
}

// Through the command itself: a token with a space or a line break inside it is refused before any request, and the message
// never shows it.
func TestImportRefusesATokenWithSpaceInsideBeforeAnyRequest(t *testing.T) {
	for _, tok := range []string{"cm_sentinel bad", "cm_sentinel\nTOKEN", " cm_sentinel\tTOKEN "} {
		srv, seen := recorder(t)
		t.Setenv(config.HomeEnv, t.TempDir())
		t.Setenv(commonlyTokenEnv, tok)
		err := importCommonly([]string{"--instance", srv.URL}, io.Discard, io.Discard)
		if err == nil || !strings.Contains(err.Error(), "spaces or line breaks") {
			t.Errorf("token of %d bytes: got %v", len(tok), err)
		}
		if err != nil && strings.Contains(err.Error(), "sentinel") {
			t.Errorf("the message holds the token: %v", err)
		}
		if n := len(seen()); n != 0 {
			t.Errorf("token of %d bytes: %d requests reached the server", len(tok), n)
		}
	}
}

// Whitespace around the token (a trailing newline from $(cat token-file), say) is trimmed by readSecret for every credential,
// so the request carries the token alone.
func TestImportTrimsWhitespaceAroundTheToken(t *testing.T) {
	srv, seen := recorder(t)
	t.Setenv(config.HomeEnv, t.TempDir())
	t.Setenv(commonlyTokenEnv, " \tcm_sentinel\n")
	err := importCommonly([]string{"--instance", srv.URL}, io.Discard, io.Discard)
	if err == nil || !strings.Contains(err.Error(), "Commonly refused this token (401)") {
		t.Fatalf("got %v", err)
	}
	if got := seen(); len(got) == 0 || got[0] != "Bearer cm_sentinel" {
		t.Fatalf("authorization headers %q, want the trimmed token", got)
	}
}
