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
	"testing"
	"time"

	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
	"github.com/Team-Commonly/jellymoon/daemon/internal/ws"
)

// silentSocket upgrades every request and then says nothing, like a half-open connection: reads on the client never return.
func silentSocket(t *testing.T) *httptest.Server {
	t.Helper()
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
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
	return srv
}

// A heartbeat that cannot be sent ends the session at once with that error, so keep reconnects, instead of leaving the
// loop blocked reading a connection that has gone half open.
func TestFailedHeartbeatEndsTheSession(t *testing.T) {
	srv := silentSocket(t)
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
