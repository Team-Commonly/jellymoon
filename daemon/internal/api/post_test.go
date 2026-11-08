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
