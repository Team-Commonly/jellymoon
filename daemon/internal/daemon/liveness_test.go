package daemon

import (
	"bufio"
	"context"
	"crypto/sha1"
	"encoding/base64"
	"io"
	"log"
	"net"
	"net/http"
	"net/http/httptest"
	"sync/atomic"
	"testing"
	"time"

	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
)

const (
	opText = 1
	opPing = 9
	opPong = 10
)

// scriptedSocket upgrades every request and hands the connection to script, which plays the pod. hits counts the connections.
func scriptedSocket(t *testing.T, script func(c net.Conn, br *bufio.Reader)) (*httptest.Server, *atomic.Int32) {
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
		script(c, buf.Reader)
	}))
	t.Cleanup(srv.Close)
	return srv, &hits
}

// clientOp reads one masked client frame and returns its opcode.
func clientOp(br *bufio.Reader) (byte, error) {
	var h [2]byte
	if _, err := io.ReadFull(br, h[:]); err != nil {
		return 0, err
	}
	n := int(h[1] & 0x7f)
	if n == 126 {
		var e [2]byte
		if _, err := io.ReadFull(br, e[:]); err != nil {
			return 0, err
		}
		n = int(e[0])<<8 | int(e[1])
	}
	if _, err := io.CopyN(io.Discard, br, int64(4+n)); err != nil { // mask, then payload
		return 0, err
	}
	return h[0] & 0x0f, nil
}

// stayConnected runs a daemon against the pod for 1.5 s (about four read timeouts) and returns how many connections it made.
func stayConnected(t *testing.T, srv *httptest.Server, hits *atomic.Int32) int32 {
	t.Helper()
	c, err := api.New(srv.URL)
	if err != nil {
		t.Fatal(err)
	}
	d := New(c, config.Store{Dir: t.TempDir()}, config.Computer{Token: "jmc_x"}, log.New(io.Discard, "", 0))
	ctx, cancel := context.WithCancel(context.Background())
	d.ctx = ctx
	d.readTimeout = 400 * time.Millisecond
	d.heartbeatEvery = 100 * time.Millisecond
	done := make(chan struct{})
	go func() { d.keep("pod_a"); close(done) }()
	time.Sleep(1500 * time.Millisecond)
	cancel()
	<-done
	return hits.Load()
}

// The ping after each heartbeat is the only sign of life on an idle socket. This pod answers a pong only to a ping that comes
// right after a heartbeat, and says nothing otherwise. With the ping the daemon stays on one connection for several timeouts;
// without it the socket looks dead every timeout and the daemon redials.
func TestPingAfterEachHeartbeatKeepsAnIdleSocketAlive(t *testing.T) {
	srv, hits := scriptedSocket(t, func(c net.Conn, br *bufio.Reader) {
		last := byte(0)
		for {
			op, err := clientOp(br)
			if err != nil {
				return
			}
			if op == opPing && last == opText {
				_, _ = c.Write([]byte{0x80 | opPong, 0})
			}
			last = op
		}
	})
	if n := stayConnected(t, srv, hits); n != 1 {
		t.Errorf("the daemon connected %d times in 1.5 s, want 1: the idle socket was treated as dead", n)
	}
}

// Any frame is proof of life, not only a pong: a pod that sends text frames and never a pong keeps the socket open too.
func TestTextFramesAloneKeepTheSocketAlive(t *testing.T) {
	srv, hits := scriptedSocket(t, func(c net.Conn, br *bufio.Reader) {
		go func() { _, _ = io.Copy(io.Discard, br) }()
		for {
			if _, err := c.Write([]byte{0x80 | opText, 2, '{', '}'}); err != nil {
				return
			}
			time.Sleep(100 * time.Millisecond)
		}
	})
	if n := stayConnected(t, srv, hits); n != 1 {
		t.Errorf("the daemon connected %d times in 1.5 s, want 1: text frames did not count as a sign of life", n)
	}
}
