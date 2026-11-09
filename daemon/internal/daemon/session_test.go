package daemon

import (
	"context"
	"crypto/sha1"
	"encoding/base64"
	"errors"
	"io"
	"log"
	"net/http"
	"net/http/httptest"
	"sync/atomic"
	"testing"
	"time"

	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
	"github.com/Team-Commonly/jellymoon/daemon/internal/ws"
)

// silentSocket upgrades every request and then says nothing, like a half-open connection: reads on the client never return.
// The count is how many connections it has accepted.
func silentSocket(t *testing.T) (*httptest.Server, *atomic.Int32) {
	t.Helper()
	var hits atomic.Int32
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		hits.Add(1)
		sum := sha1.Sum([]byte(r.Header.Get("Sec-WebSocket-Key") + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"))
		c, buf, err := w.(http.Hijacker).Hijack()
		if err != nil {
			return
		}
		defer c.Close()
		_, _ = buf.WriteString("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: " +
			base64.StdEncoding.EncodeToString(sum[:]) + "\r\n\r\n")
		_ = buf.Flush()
		_, _ = io.Copy(io.Discard, c)
	}))
	t.Cleanup(srv.Close)
	return srv, &hits
}

// A heartbeat that cannot be sent ends the session at once with that error, so keep reconnects, instead of leaving the
// loop blocked reading a connection that has gone half open.
func TestFailedHeartbeatEndsTheSession(t *testing.T) {
	srv, _ := silentSocket(t)
	c, err := api.New(srv.URL)
	if err != nil {
		t.Fatal(err)
	}
	d := New(c, config.Store{Dir: t.TempDir()}, config.Computer{Token: "jmc_x"}, log.New(io.Discard, "", 0))
	ctx, cancel := context.WithCancel(context.Background())
	t.Cleanup(cancel)
	d.ctx = ctx
	boom := errors.New("heartbeat send failed")
	d.beat = func(*ws.Conn, []byte) error { return boom }
	done := make(chan error, 1)
	go func() { done <- d.session("pod_a") }()
	select {
	case err := <-done:
		if !errors.Is(err, boom) {
			t.Errorf("session returned %v, want the heartbeat error", err)
		}
	case <-time.After(3 * time.Second):
		t.Fatal("the session is still blocked reading after its heartbeat failed")
	}
}

// A connection that goes silent without closing (no close frame, no FIN) is dropped after the read timeout and dialled again,
// instead of leaving the daemon deaf to wakes while the pod calls the computer offline.
func TestSilentSocketIsReconnected(t *testing.T) {
	srv, hits := silentSocket(t)
	c, err := api.New(srv.URL)
	if err != nil {
		t.Fatal(err)
	}
	d := New(c, config.Store{Dir: t.TempDir()}, config.Computer{Token: "jmc_x"}, log.New(io.Discard, "", 0))
	ctx, cancel := context.WithCancel(context.Background())
	d.ctx = ctx
	d.readTimeout = 300 * time.Millisecond
	done := make(chan struct{})
	go func() { d.keep("pod_a"); close(done) }()
	t.Cleanup(func() { cancel(); <-done })
	deadline := time.After(6 * time.Second) // the 300 ms timeout, the 1 s reconnect wait and the second dial, with room
	for hits.Load() < 2 {
		select {
		case <-deadline:
			t.Fatalf("the daemon never reconnected: %d connection(s) after 6 s of silence", hits.Load())
		case <-time.After(50 * time.Millisecond):
		}
	}
}
