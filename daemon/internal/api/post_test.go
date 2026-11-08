package api

import (
	"fmt"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

// cutShort answers with a status and a Content-Length it does not keep: it sends `sent` and drops the connection, so the
// client's read of the body fails after the response began.
func cutShort(status int, sent string) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		conn, buf, err := w.(http.Hijacker).Hijack()
		if err != nil {
			return
		}
		defer conn.Close()
		_, _ = fmt.Fprintf(buf, "HTTP/1.1 %d X\r\nContent-Type: application/json\r\nContent-Length: 500\r\n\r\n%s", status, sent)
		_ = buf.Flush()
	}
}

// A request whose response body breaks mid-read is an error, not an empty success and not a half-decoded value.
func TestPostReturnsTheReadErrorOfABrokenBody(t *testing.T) {
	cases := map[string]struct {
		status int
		sent   string
	}{
		"success, nothing sent":  {200, ""},
		"success, partial JSON":  {200, `{"agentId":"agt_`},
		"failure, nothing sent":  {500, ""},
		"failure, partial error": {400, `{"code":"invalid_input","mess`},
	}
	for name, c := range cases {
		t.Run(name, func(t *testing.T) {
			srv := httptest.NewServer(cutShort(c.status, c.sent))
			defer srv.Close()
			cl, err := New(srv.URL)
			if err != nil {
				t.Fatal(err)
			}
			id, err := cl.RegisterAgent("jmc_x", "pod_a", protocol.HarnessFake, "a")
			if err == nil {
				t.Fatalf("no error for a response cut mid-body (agent id %q)", id)
			}
			if !strings.Contains(err.Error(), "reading the response") {
				t.Errorf("error %q does not say the response could not be read", err)
			}
		})
	}
}

// A 2xx with an empty body is not a result when the caller needs one: a claim must not come back as an unset response.
// A call that passes no result (EndRun) still succeeds on an empty body.
func TestPostRefusesAnEmptyBodyWhenAResultIsNeeded(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.WriteHeader(http.StatusOK) }))
	defer srv.Close()
	cl, err := New(srv.URL)
	if err != nil {
		t.Fatal(err)
	}
	if got, err := cl.ClaimRun("jmc_x", "wak_x"); err == nil {
		t.Errorf("ClaimRun accepted an empty 200 body and returned %+v", got)
	}
	if err := cl.EndRun("jmc_x", "run_x", "completed"); err != nil {
		t.Errorf("EndRun, which wants no result, failed on an empty body: %v", err)
	}
}
