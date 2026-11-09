package importer

import (
	"net"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync/atomic"
	"testing"
	"time"
)

type countingTransport struct{ n atomic.Int32 }

func (c *countingTransport) RoundTrip(r *http.Request) (*http.Response, error) {
	c.n.Add(1)
	return http.DefaultTransport.RoundTrip(r)
}

func clientFor(t *testing.T, base string) (*Commonly, *countingTransport, *[]time.Duration) {
	t.Helper()
	c, err := NewCommonly(base, "cm_sentinelTOKEN0001")
	if err != nil {
		t.Fatal(err)
	}
	ct := &countingTransport{}
	c.hc.Transport = ct
	var slept []time.Duration
	c.sleep = func(d time.Duration) { slept = append(slept, d) }
	c.gap = 0
	return c, ct, &slept
}

func closedAddr(t *testing.T) string {
	t.Helper()
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	a := l.Addr().String()
	l.Close()
	return a
}

// A refused connection on the identity check fails at once, with one attempt, naming the host and never the token.
func TestIdentityCheckOnAClosedPortFailsAtOnce(t *testing.T) {
	addr := closedAddr(t)
	c, ct, slept := clientFor(t, "http://"+addr)
	_, err := whoAmI(c)
	if err == nil || !strings.Contains(err.Error(), "can't reach "+addr+": connection refused") {
		t.Fatalf("got %v", err)
	}
	if ct.n.Load() != 1 || len(*slept) != 0 {
		t.Fatalf("%d attempts, waits %v", ct.n.Load(), *slept)
	}
	if strings.Contains(err.Error(), "sentinel") || strings.Contains(err.Error(), "http://") {
		t.Fatalf("message leaks: %v", err)
	}
}

// A name that does not resolve is treated the same way.
func TestIdentityCheckOnAnUnresolvableHostFailsAtOnce(t *testing.T) {
	c, ct, slept := clientFor(t, "https://commonly-nowhere.invalid")
	_, err := whoAmI(c)
	if err == nil || !strings.Contains(err.Error(), "can't reach commonly-nowhere.invalid") || !strings.Contains(err.Error(), "doesn't resolve") {
		t.Fatalf("got %v", err)
	}
	if ct.n.Load() != 1 || len(*slept) != 0 {
		t.Fatalf("%d attempts, waits %v", ct.n.Load(), *slept)
	}
	if strings.Contains(err.Error(), "sentinel") {
		t.Fatalf("message leaks: %v", err)
	}
}

// After the instance has answered once, a refused connection is transient again: retried, and stopped after five.
func TestRefusalLaterInARunStillRetries(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { _, _ = w.Write([]byte(`{}`)) }))
	c, ct, _ := clientFor(t, srv.URL)
	if err := c.Get("/api/auth/user", nil, &Me{}); err != nil {
		t.Fatal(err)
	}
	srv.Close()
	err := c.Get("/api/pods", nil, nil)
	if err != ErrStopped || ct.n.Load() != 6 {
		t.Fatalf("err %v after %d attempts", err, ct.n.Load())
	}
}

// The stop message makes no claim about saved state; the command adds that only when a state file exists.
func TestErrStoppedMakesNoClaimAboutState(t *testing.T) {
	if strings.Contains(ErrStopped.Error(), "saved") {
		t.Fatalf("ErrStopped claims a save: %v", ErrStopped)
	}
}
