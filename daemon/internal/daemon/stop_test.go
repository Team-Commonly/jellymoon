package daemon

import (
	"bufio"
	"context"
	"io"
	"log"
	"net"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

// lockedBuf is a log sink two goroutines can write and the test can read.
type lockedBuf struct {
	mu sync.Mutex
	b  strings.Builder
}

func (l *lockedBuf) Write(p []byte) (int, error) {
	l.mu.Lock()
	defer l.mu.Unlock()
	return l.b.Write(p)
}

func (l *lockedBuf) String() string {
	l.mu.Lock()
	defer l.mu.Unlock()
	return l.b.String()
}

// runAgainst runs a whole daemon with one agent whose pod plays script, and returns its stop, its log and Run's result.
func runAgainst(t *testing.T, script func(c net.Conn, br *bufio.Reader)) (stop func(), logs *lockedBuf, result chan error) {
	t.Helper()
	srv, _ := scriptedSocket(t, script)
	c, err := api.New(srv.URL)
	if err != nil {
		t.Fatal(err)
	}
	store := config.Store{Dir: t.TempDir()}
	if err := store.AddAgent(config.Agent{PodID: "pod_a", AgentID: "agt_1", Harness: protocol.HarnessFake, DisplayName: "a"}); err != nil {
		t.Fatal(err)
	}
	logs = &lockedBuf{}
	d := New(c, store, config.Computer{Token: "jmc_x"}, log.New(logs, "", 0))
	ctx, cancel := context.WithCancel(context.Background())
	result = make(chan error, 1)
	go func() { result <- d.Run(ctx) }()
	t.Cleanup(cancel)
	return cancel, logs, result
}

func waitFor(t *testing.T, what string, ok func() bool) {
	t.Helper()
	for end := time.Now().Add(3 * time.Second); time.Now().Before(end); time.Sleep(10 * time.Millisecond) {
		if ok() {
			return
		}
	}
	t.Fatalf("timed out waiting for %s", what)
}

// A stop (Ctrl-C, SIGTERM) closes the pod sockets without calling that a failure: no reconnect line, one "stopped" line, and
// Run returns nil, which the CLI turns into exit 0.
func TestStopClosesSocketsQuietlyAndSaysStopped(t *testing.T) {
	stop, logs, result := runAgainst(t, func(c net.Conn, br *bufio.Reader) { _, _ = io.Copy(io.Discard, br) })
	waitFor(t, "the pod connection", func() bool { return strings.Contains(logs.String(), "pod pod_a: connected") })
	stop()
	select {
	case err := <-result:
		if err != nil {
			t.Fatalf("Run = %v, want nil", err)
		}
	case <-time.After(3 * time.Second):
		t.Fatal("Run did not return after the stop")
	}
	out := logs.String()
	if strings.Contains(out, "reconnecting") || strings.Contains(out, "socket ended") {
		t.Errorf("a clean stop logged a failure:\n%s", out)
	}
	if strings.Count(out, "stopped") != 1 {
		t.Errorf("want one \"stopped\" line:\n%s", out)
	}
}

// A socket that ends while the daemon is not stopping still logs the reconnect line and dials again.
func TestUnexpectedSocketEndStillReconnects(t *testing.T) {
	_, logs, _ := runAgainst(t, func(c net.Conn, br *bufio.Reader) { _ = c.Close() })
	waitFor(t, "the reconnect line", func() bool {
		return strings.Contains(logs.String(), "socket ended") && strings.Contains(logs.String(), "reconnecting in 1s")
	})
	if strings.Contains(logs.String(), "stopped") {
		t.Errorf("logged stopped while running:\n%s", logs.String())
	}
}
