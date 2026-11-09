package importer

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"sync/atomic"
	"testing"
)

// The identity check names what went wrong, and no message holds the token.
func TestWhoAmIExplainsARefusalAndAWrongHost(t *testing.T) {
	cases := []struct {
		status int
		want   string
	}{
		{401, "Commonly refused this token (401): check COMMONLY_TOKEN"},
		{403, "Commonly refused this token (403): check COMMONLY_TOKEN"},
		{404, "doesn't look like a Commonly instance (no /api/auth/user)"},
		{418, "127.0.0.1"},
	}
	for _, tc := range cases {
		status := tc.status
		c, _ := newTestClient(t, func(w http.ResponseWriter, r *http.Request) { w.WriteHeader(status) })
		_, err := whoAmI(c)
		if err == nil || !strings.Contains(err.Error(), tc.want) {
			t.Errorf("%d: got %v, want it to contain %q", tc.status, err, tc.want)
			continue
		}
		if strings.Contains(err.Error(), "sentinel") {
			t.Errorf("%d: the message holds the token: %v", tc.status, err)
		}
		if tc.status == 404 && !strings.Contains(err.Error(), "http://127.0.0.1") {
			t.Errorf("404 message does not name the url: %v", err)
		}
	}
}

// A token with whitespace or a control character is refused before any request, and the message never shows it.
func TestNewCommonlyRefusesAMalformedTokenLocally(t *testing.T) {
	var hits atomic.Int32
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { hits.Add(1) }))
	t.Cleanup(srv.Close)
	for _, tok := range []string{"cm_ bad token with spaces", "cm_sentinel\nTOKEN", "cm_sentinel\tTOKEN", "cm_sentinel\x00TOKEN", "cm_sentinel\r"} {
		_, err := NewCommonly(srv.URL, tok)
		if err == nil || err.Error() != "COMMONLY_TOKEN has spaces or line breaks in it; paste the token alone" {
			t.Errorf("token of %d bytes: got %v", len(tok), err)
			continue
		}
		if strings.Contains(err.Error(), "sentinel") || strings.Contains(err.Error(), "bad") {
			t.Errorf("message holds the token: %v", err)
		}
	}
	if hits.Load() != 0 {
		t.Fatalf("%d requests reached the server", hits.Load())
	}
}
