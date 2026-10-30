// Package e2e is the black-box contract suite: it runs unchanged against any server that implements
// docs/protocol.md plus the test-route contract in docs/development/e2e-contract.md.
package e2e

import (
	"bytes"
	"fmt"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"slices"
	"sync"
	"syscall"
	"testing"
	"time"
)

const (
	leaseMs = 60000
	// fakeBase is 2040-01-01T00:00:00Z. It is far in the future so that a restarted server running on its
	// real clock never finds a lease due before the suite sets the clock again.
	fakeBase = int64(2208988800000)
)

var (
	suiteStart = time.Now()
	serverCmd  = os.Getenv("JM_E2E_SERVER")
	realClock  = os.Getenv("JM_E2E_REAL_CLOCK") == "1"
	suiteTests = []string{"TestF02", "TestF07", "TestF10", "TestExpiryZeroTraffic", "TestRenewRace",
		"TestKillMidLease", "TestReplayAcrossRestart", "TestOneReleasePerCause", "TestInputValidation", "TestRestartPastDeadline"}
	outcomes sync.Map // test name -> failed bool
)

func TestMain(m *testing.M) {
	// staleMs is 0 (ADR 0003, "What an adapter must do", item 5): every command is persisted, heartbeats included.
	// The knob that once widened the early-expiry bound is gone, and a value other than 0 is an error, not ignored.
	if v := os.Getenv("JM_E2E_STALE_MS"); v != "" && v != "0" {
		fmt.Fprintln(os.Stderr, "JM_E2E_STALE_MS is no longer a setting: staleMs must be 0 (docs/adr/0003-pure-pod-core.md, item 5)")
		os.Exit(2)
	}
	code := m.Run()
	if serverCmd != "" {
		fmt.Println(summaryLine(time.Since(suiteStart).Milliseconds(), stepsOK.Load(), stepsRun.Load()))
	}
	os.Exit(code)
}

func need(t *testing.T) {
	t.Helper()
	if serverCmd == "" {
		t.Skip("JM_E2E_SERVER is not set; run `make e2e` with a server command (docs/development/e2e-contract.md)")
	}
}

// needFake skips tests that drive the clock; real-clock mode never calls /__test/clock.
func needFake(t *testing.T) {
	t.Helper()
	need(t)
	if realClock {
		t.Skip("JM_E2E_REAL_CLOCK=1: only TestExpiryZeroTraffic and TestKillMidLease run in real-clock mode")
	}
}

func track(t *testing.T) {
	t.Cleanup(func() { outcomes.Store(t.Name(), t.Failed()) })
}

type logBuf struct {
	mu sync.Mutex
	b  bytes.Buffer
}

func (l *logBuf) Write(p []byte) (int, error) {
	l.mu.Lock()
	defer l.mu.Unlock()
	return l.b.Write(p)
}

func (l *logBuf) tail(n int) string {
	l.mu.Lock()
	defer l.mu.Unlock()
	b := l.b.Bytes()
	return string(b[max(0, len(b)-n*100):])
}

// server is the process under test. The suite owns it: kill -9 of its process group, then the same
// command again with the same data dir, is a restart.
type server struct {
	t          *testing.T
	port       int
	inspector  int // a second port the server may use for its debugger; never shared between servers
	dir        string
	cmd        *exec.Cmd
	exited     chan struct{}
	log        *logBuf
	restarting bool
	bootTook   time.Duration // from first start to the first 200 from /healthz
}

// freePort returns a port nobody listens on, and never one in `not`.
func freePort(t *testing.T, not ...int) int {
	for {
		l, err := net.Listen("tcp", "127.0.0.1:0")
		if err != nil {
			t.Fatalf("no free port: %v", err)
		}
		p := l.Addr().(*net.TCPAddr).Port
		l.Close()
		if !slices.Contains(not, p) {
			return p
		}
	}
}

func startServer(t *testing.T) *server {
	port := freePort(t)
	s := &server{t: t, port: port, inspector: freePort(t, port), dir: t.TempDir(), log: &logBuf{}}
	t.Cleanup(func() {
		s.kill()
		if t.Failed() {
			t.Logf("server log tail:\n%s", s.log.tail(40))
		}
	})
	s.boot()
	return s
}

func (s *server) base() string { return fmt.Sprintf("http://127.0.0.1:%d", s.port) }

func (s *server) boot() {
	begin := time.Now()
	root, _ := filepath.Abs("..")
	cmd := exec.Command("sh", "-c", serverCmd)
	cmd.Dir = root
	cmd.Env = append(os.Environ(), fmt.Sprintf("PORT=%d", s.port), fmt.Sprintf("INSPECTOR_PORT=%d", s.inspector), "ENV=local", "JM_DATA_DIR="+s.dir)
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	cmd.Stdout, cmd.Stderr = s.log, s.log
	if err := cmd.Start(); err != nil {
		s.t.Fatalf("cannot start JM_E2E_SERVER: %v", err)
	}
	s.cmd, s.exited = cmd, make(chan struct{})
	go func(c *exec.Cmd, ch chan struct{}) { _ = c.Wait(); close(ch) }(cmd, s.exited)
	s.waitHealthy()
	if s.bootTook == 0 {
		s.bootTook = time.Since(begin)
	}
}

func (s *server) waitHealthy() {
	hc := &http.Client{Timeout: 2 * time.Second}
	limit := 40 * time.Second
	if s.restarting {
		limit = 10 * time.Second
	}
	deadline := time.Now().Add(limit)
	for time.Now().Before(deadline) {
		select {
		case <-s.exited:
			s.t.Fatalf("server exited before /healthz answered 200; log tail:\n%s", s.log.tail(20))
		default:
		}
		if resp, err := hc.Get(s.base() + "/healthz"); err == nil {
			resp.Body.Close()
			if resp.StatusCode == 200 {
				return
			}
		}
		time.Sleep(50 * time.Millisecond)
	}
	msg := "server did not answer GET /healthz with 200 within 40 s"
	if s.restarting {
		msg = "E2E_INVALID restart took over 10 s to answer /healthz (not a server failure)"
		fmt.Println("E2E_INVALID restart took over 10 s to answer /healthz")
	}
	s.t.Fatalf("%s; log tail:\n%s", msg, s.log.tail(20))
}

func (s *server) kill() {
	if s.cmd == nil || s.cmd.Process == nil {
		return
	}
	pgid := s.cmd.Process.Pid
	_ = syscall.Kill(-pgid, syscall.SIGKILL)
	<-s.exited
	// The sh we started is only the group leader. A server that forks (wrangler, then workerd) outlives it, so wait until
	// no process is left in the group; a restart on the same port must not meet the old server still listening.
	deadline := time.Now().Add(15 * time.Second)
	for syscall.Kill(-pgid, 0) == nil {
		if time.Now().After(deadline) {
			s.t.Fatalf("processes of the server's group %d still run 15 s after kill -9", pgid)
		}
		_ = syscall.Kill(-pgid, syscall.SIGKILL)
		time.Sleep(20 * time.Millisecond)
	}
}

// crash is the injected restart: kill -9 of the process group, an optional hold with the server down
// (real-clock mode uses it to let a deadline fall inside the outage), then the same command on the same
// port and data dir. It returns the wall time in unix ms at which /healthz answered. A boot slower than
// 10 s prints `E2E_INVALID <reason>` and fails the run as INVALID, which is not a server failure.
func (w *world) crash(holdUntil int64) int64 {
	w.t.Helper()
	w.srv.kill()
	if d := holdUntil - time.Now().UnixMilli(); realClock && d > 0 {
		time.Sleep(time.Duration(d) * time.Millisecond)
	}
	w.srv.restarting = true
	w.srv.boot()
	w.srv.restarting = false
	return time.Now().UnixMilli()
}
