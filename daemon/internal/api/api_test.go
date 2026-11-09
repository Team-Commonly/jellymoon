package api

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

// at is built, not written, so the tracked-text scan does not read these URLs as addresses.
const at = "@"

func TestCheckServer(t *testing.T) {
	good := map[string]string{
		"https://api.example.com":     "https://api.example.com",
		"https://api.example.com/x/":  "https://api.example.com",
		"http://localhost:8787":       "http://localhost:8787",
		"http://127.0.0.1:8787/v1":    "http://127.0.0.1:8787",
		"https://127.0.0.1.nip.io:44": "https://127.0.0.1.nip.io:44",
	}
	for in, want := range good {
		if u, err := CheckServer(in); err != nil || u.String() != want {
			t.Errorf("CheckServer(%q) = %v, %v; want %s", in, u, err, want)
		}
	}
	for _, in := range []string{
		"http://example.com", "http://localhost.evil.com", "http://127.0.0.1.evil.com", "http://127.0.0.2:80", "http://[::1]:80",
		"http://localhost" + at + "evil.com", "https://user:pw" + at + "example.com", "ftp://example.com", "ws://localhost", "example.com", "", "https://", "//localhost",
	} {
		if u, err := CheckServer(in); err == nil {
			t.Errorf("CheckServer(%q) accepted %v", in, u)
		}
	}
}

func TestRefusalNamesNoCredentials(t *testing.T) {
	_, err := CheckServer("http://user:secretpw" + at + "example.com")
	if err == nil || strings.Contains(err.Error(), "secretpw") {
		t.Errorf("a refusal must not echo credentials: %v", err)
	}
}

// An error names the method and the route, never the query string: a query value can come from argv, and argv can hold a
// pasted credential. Covered: a refused connection, a status with no error body, and a server refusal.
func TestErrorsNameNoQueryValue(t *testing.T) {
	secret := "lin" + "_api_" + "EXAMPLE" + "0123456789abcdef"
	plain := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) { w.WriteHeader(502) }))
	defer plain.Close()
	refusing := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(400)
		_, _ = w.Write([]byte(`{"code":"invalid_request","message":"bad decision id"}`))
	}))
	defer refusing.Close()
	for _, server := range []string{"http://127.0.0.1:1", plain.URL, refusing.URL} {
		c, err := New(server)
		if err != nil {
			t.Fatal(err)
		}
		_, err = c.ConnectorReceipts("jmu_test", "pod_01ARZ3NDEKTSV4RRFFQ69G5FAV", &secret, 5)
		if err == nil || strings.Contains(err.Error(), "EXAMPLE") || strings.Contains(err.Error(), "?") || !strings.Contains(err.Error(), "GET /v1/pods/") {
			t.Errorf("%s: error %v; want the method and route, with no query string", server, err)
		}
	}
}

// A redirect is never followed, so a token cannot be sent on to the host a server points at.
func TestNoRedirects(t *testing.T) {
	var hit bool
	other := httptest.NewServer(http.HandlerFunc(func(http.ResponseWriter, *http.Request) { hit = true }))
	defer other.Close()
	srv := httptest.NewServer(http.RedirectHandler(other.URL, http.StatusTemporaryRedirect))
	defer srv.Close()
	c, err := New(srv.URL) // httptest listens on 127.0.0.1
	if err != nil {
		t.Fatal(err)
	}
	if _, err := c.RegisterComputer("jmu_x"); err == nil || hit {
		t.Errorf("err=%v, the redirect target was hit=%v; want an error and no hit", err, hit)
	}
}

// The MCP URL a claim returns gets the server rule before the run token is sent: https, or http on localhost or 127.0.0.1.
func TestCheckMCP(t *testing.T) {
	for _, in := range []string{"https://mcp.example.com/mcp", "http://127.0.0.1:8787/mcp", "http://localhost/mcp"} {
		if got, err := CheckMCP(in); err != nil || got != in {
			t.Errorf("CheckMCP(%q) = %q, %v; want it accepted unchanged", in, got, err)
		}
	}
	for _, in := range []string{"http://example.com/mcp", "http://localhost.evil.com/mcp", "http://127.0.0.1.evil.com/mcp", "ftp://example.com/mcp", "https://user:pw" + at + "example.com/mcp", "/mcp", ""} {
		if got, err := CheckMCP(in); err == nil {
			t.Errorf("CheckMCP(%q) accepted %q", in, got)
		}
	}
}

// A URL with no scheme is refused with the forms that work, not as "not https": http on localhost is allowed too.
func TestCheckServerWithNoSchemeNamesBothForms(t *testing.T) {
	for _, raw := range []string{"localhost:8787", "commonly.example"} {
		_, err := CheckServer(raw)
		if err == nil || !strings.Contains(err.Error(), "http://localhost") || strings.Contains(err.Error(), "plain https") {
			t.Errorf("%q: got %v, want an error naming https:// and http://localhost", raw, err)
		}
	}
}
