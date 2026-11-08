package ws

import (
	"context"
	"crypto/sha1"
	"encoding/base64"
	"errors"
	"net"
	"net/http"
	"net/http/httptest"
	"net/url"
	"os"
	"testing"
	"time"
)

// serve upgrades every request and hands the raw connection to script, which plays the pod.
func serve(t *testing.T, script func(c net.Conn)) *url.URL {
	t.Helper()
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		sum := sha1.Sum([]byte(r.Header.Get("Sec-WebSocket-Key") + guid))
		c, buf, err := w.(http.Hijacker).Hijack()
		if err != nil {
			return
		}
		defer c.Close()
		_, _ = buf.WriteString("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: " +
			base64.StdEncoding.EncodeToString(sum[:]) + "\r\n\r\n")
		_ = buf.Flush()
		script(c)
	}))
	t.Cleanup(srv.Close)
	u, _ := url.Parse(srv.URL)
	return u
}

func dial(t *testing.T, u *url.URL) *Conn {
	t.Helper()
	c, err := Dial(context.Background(), u, "jmc_x")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = c.Close() })
	return c
}

// serverFrame is one unmasked frame, as a pod sends it.
func serverFrame(op byte, p string) []byte { return append([]byte{0x80 | op, byte(len(p))}, p...) }

// A socket that says nothing, with no close frame and no FIN, makes Read fail once the timeout passes.
func TestReadTimesOutOnASilentSocket(t *testing.T) {
	hold := make(chan struct{})
	t.Cleanup(func() { close(hold) })
	c := dial(t, serve(t, func(net.Conn) { <-hold }))
	c.ReadTimeout = 200 * time.Millisecond
	done := make(chan error, 1)
	go func() { _, err := c.Read(); done <- err }()
	select {
	case err := <-done:
		if !errors.Is(err, os.ErrDeadlineExceeded) {
			t.Errorf("Read = %v, want a deadline error", err)
		}
	case <-time.After(3 * time.Second):
		t.Fatal("Read is still blocked on a silent socket")
	}
}

// Any frame is proof of life, a pong included: a server that only pongs keeps the socket open past the timeout, and the
// message that finally arrives is delivered.
func TestAPongKeepsTheSocketAlive(t *testing.T) {
	c := dial(t, serve(t, func(s net.Conn) {
		for i := 0; i < 15; i++ { // 1.5 s of pongs against a 500 ms timeout
			_, _ = s.Write(serverFrame(opPong, ""))
			time.Sleep(100 * time.Millisecond)
		}
		_, _ = s.Write(serverFrame(opText, "hi"))
		time.Sleep(time.Second)
	}))
	c.ReadTimeout = 500 * time.Millisecond
	got, err := c.Read()
	if err != nil || string(got) != "hi" {
		t.Errorf("Read = %q, %v; want hi", got, err)
	}
}
