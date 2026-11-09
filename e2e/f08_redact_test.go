package e2e

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

// TestF08Redaction: a server's error text reaches the CLI's output with a token-shaped string shortened and the escape
// sequence gone. The server here is a stub on 127.0.0.1 that refuses the computer registration with such a message; the
// daemon is the repo's own binary, run as a black box.
func TestF08Redaction(t *testing.T) {
	need(t)
	par(t)
	track(t)
	const planted = "jmu_PlantedByTheServer0123"
	stub := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		_, _ = w.Write([]byte(`{"code":"invalid_input","message":"bad token ` + planted + ` \u001b[2J\u001b]0;pwned\u0007 end\r"}`))
	}))
	defer stub.Close()
	env := []string{"JELLYMOON_HOME=" + t.TempDir(), "JELLYMOON_SERVER=" + stub.URL, "JELLYMOON_USER_TOKEN=jmu_TheUsersOwnToken9876"}
	p := spawn(t, buildDaemon(t), env, "daemon", "start", "--foreground")
	code, ok := p.finish(10 * time.Second)
	out := p.text()
	if !ok || code == 0 {
		t.Fatalf("exit %d finished=%v; a refused registration must end the start with an error:\n%q", code, ok, out)
	}
	if !strings.Contains(out, "bad token jmu_… [2J]0;pwned end") {
		t.Errorf("the server's message did not arrive redacted (token shortened, escape and CR gone): %q", out)
	}
	if strings.Contains(out, "PlantedByTheServer") || strings.Contains(out, "TheUsersOwnToken") || strings.ContainsAny(out, "\x1b\r\x07") {
		t.Errorf("output holds a token tail or a control character: %q", out)
	}
}
