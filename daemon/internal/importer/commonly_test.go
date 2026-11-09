package importer

import (
	"net/http"
	"net/http/httptest"
	"net/url"
	"strings"
	"sync/atomic"
	"testing"
	"time"
)

func newTestClient(t *testing.T, h http.HandlerFunc) (*Commonly, *[]time.Duration) {
	t.Helper()
	srv := httptest.NewServer(h)
	t.Cleanup(srv.Close)
	c, err := NewCommonly(srv.URL, "cm_sentinelTOKEN0001")
	if err != nil {
		t.Fatal(err)
	}
	var slept []time.Duration
	c.sleep = func(d time.Duration) { slept = append(slept, d) }
	c.gap = 0
	return c, &slept
}

func TestInstanceMustBeHTTPSOrLoopback(t *testing.T) {
	for _, ok := range []string{"https://commonly.example", "http://127.0.0.1:4000", "http://localhost:4000"} {
		if _, err := NewCommonly(ok, "t"); err != nil {
			t.Errorf("%s refused: %v", ok, err)
		}
	}
	for _, bad := range []string{"http://commonly.example", "ftp://x", "https://u:p@commonly.example", ""} {
		if _, err := NewCommonly(bad, "t"); err == nil {
			t.Errorf("%q accepted", bad)
		}
	}
}

// Only the allowlisted GET routes are ever called, and no scope parameter is ever sent: the refusal is in the client, before any request.
func TestGetRefusesWhatTheImporterMustNeverCall(t *testing.T) {
	var hits atomic.Int32
	c, _ := newTestClient(t, func(w http.ResponseWriter, r *http.Request) { hits.Add(1) })
	for _, p := range []string{"/api/users/abc", "/api/credentials", "/api/agents/runtime/pods/x/context", "/api/integrations/x/ingest-tokens", "/api/uploads/f", "/api/pods/x/members"} {
		if err := c.Get(p, nil, nil); err == nil {
			t.Errorf("%s was allowed", p)
		}
	}
	for _, k := range []string{"scope", "Scope", "SCOPE"} {
		if err := c.Get("/api/pods", url.Values{k: {"all"}}, nil); err == nil {
			t.Errorf("query %s was allowed", k)
		}
	}
	if hits.Load() != 0 {
		t.Fatalf("%d requests reached the server", hits.Load())
	}
}

func TestGetSendsTheTokenAsBearerOnly(t *testing.T) {
	c, _ := newTestClient(t, func(w http.ResponseWriter, r *http.Request) {
		if r.Header.Get("Authorization") != "Bearer cm_sentinelTOKEN0001" || r.URL.RawQuery != "" {
			t.Errorf("auth %q query %q", r.Header.Get("Authorization"), r.URL.RawQuery)
		}
		_, _ = w.Write([]byte(`{"role":"user"}`))
	})
	var me Me
	if err := c.Get("/api/auth/user", nil, &me); err != nil || me.Role != "user" {
		t.Fatalf("%v %+v", err, me)
	}
}

// A 429 with Retry-After, then a 503, then success: the wait honours Retry-After and the second wait is jittered backoff.
func TestGetBacksOffOn429Then503(t *testing.T) {
	var n atomic.Int32
	c, slept := newTestClient(t, func(w http.ResponseWriter, r *http.Request) {
		switch n.Add(1) {
		case 1:
			w.Header().Set("Retry-After", "7")
			w.WriteHeader(429)
		case 2:
			w.WriteHeader(503)
		default:
			_, _ = w.Write([]byte(`{"role":"user"}`))
		}
	})
	if err := c.Get("/api/auth/user", nil, &Me{}); err != nil {
		t.Fatal(err)
	}
	if len(*slept) != 2 || (*slept)[0] < 7*time.Second || (*slept)[1] <= 0 || (*slept)[1] > 2*time.Second {
		t.Fatalf("waits %v", *slept)
	}
}

func TestGetStopsAfterFiveConsecutiveFailures(t *testing.T) {
	var n atomic.Int32
	c, _ := newTestClient(t, func(w http.ResponseWriter, r *http.Request) { n.Add(1); w.WriteHeader(503) })
	err := c.Get("/api/pods", nil, nil)
	if err != ErrStopped || n.Load() != 5 {
		t.Fatalf("err %v after %d requests", err, n.Load())
	}
}

// A success resets the count: four failures, a success, and four more failures do not stop the run.
func TestGetFailureCountResetsOnSuccess(t *testing.T) {
	var n atomic.Int32
	c, _ := newTestClient(t, func(w http.ResponseWriter, r *http.Request) {
		if k := n.Add(1); k == 5 || k >= 10 {
			_, _ = w.Write([]byte(`{}`))
			return
		}
		w.WriteHeader(500)
	})
	for i := 0; i < 2; i++ {
		if err := c.Get("/api/pods", nil, &struct{}{}); err != nil {
			t.Fatalf("call %d: %v", i, err)
		}
	}
}

func TestGetDoesNotFollowARedirectToAnotherHost(t *testing.T) {
	var other atomic.Int32
	o := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { other.Add(1) }))
	defer o.Close()
	c, _ := newTestClient(t, func(w http.ResponseWriter, r *http.Request) { http.Redirect(w, r, o.URL+"/steal", http.StatusFound) })
	if err := c.Get("/api/pods", nil, nil); err == nil || other.Load() != 0 {
		t.Fatalf("err %v, other host hit %d times", err, other.Load())
	}
}

func TestErrorsNeverCarryTheTokenOrTheBody(t *testing.T) {
	c, _ := newTestClient(t, func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(403)
		_, _ = w.Write([]byte(`{"error":"cm_sentinelTOKEN0001 leaked in a body"}`))
	})
	err := c.Get("/api/pods/p1", nil, nil)
	if err == nil || strings.Contains(err.Error(), "sentinel") || strings.Contains(err.Error(), "leaked") {
		t.Fatalf("%v", err)
	}
}

// Retry-After in HTTP-date form is honoured too, capped like the seconds form.
func TestGetHonoursARetryAfterDate(t *testing.T) {
	var n atomic.Int32
	c, slept := newTestClient(t, func(w http.ResponseWriter, r *http.Request) {
		if n.Add(1) == 1 {
			w.Header().Set("Retry-After", time.Now().Add(20*time.Second).UTC().Format(http.TimeFormat))
			w.WriteHeader(429)
			return
		}
		_, _ = w.Write([]byte(`{}`))
	})
	if err := c.Get("/api/pods", nil, &struct{}{}); err != nil {
		t.Fatal(err)
	}
	if len(*slept) != 1 || (*slept)[0] < 15*time.Second || (*slept)[0] > retryAfterCap {
		t.Fatalf("waits %v", *slept)
	}
}
