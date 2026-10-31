// Package e2e is the black-box contract suite: it runs unchanged against any server that implements
// docs/protocol.md plus the test-route contract in docs/development/e2e-contract.md.
package e2e

import (
	"bufio"
	"bytes"
	"fmt"
	"net"
	"net/http"
	"net/url"
	"os"
	"os/exec"
	"path/filepath"
	"slices"
	"strings"
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
	// remoteURL is a deployed server the suite does not boot (docs/development/e2e-contract.md, "Remote mode"). It implies the real clock.
	remoteURL    = strings.TrimRight(os.Getenv("JM_E2E_URL"), "/")
	remoteBearer = os.Getenv("JM_E2E_TEST_BEARER")
	realClock    = os.Getenv("JM_E2E_REAL_CLOCK") == "1" || remoteURL != ""
	suiteTests   = []string{"TestF02", "TestF03", "TestF04", "TestF05", "TestF06", "TestF07", "TestF08", "TestF10", "TestExpiryZeroTraffic", "TestRenewRace",
		"TestKillMidLease", "TestReplayAcrossRestart", "TestOneReleasePerCause", "TestInputValidation", "TestRestartPastDeadline", "TestRunTokenOtherPod", "TestEvictMidClaim", "TestEvictAlarmWake", "TestEvictPastDeadline"}
	outcomes sync.Map // test name -> failed bool
)

func TestMain(m *testing.M) {
	// staleMs is 0 (ADR 0003, "What an adapter must do", item 5): every command is persisted, heartbeats included.
	// The knob that once widened the early-expiry bound is gone, and a value other than 0 is an error, not ignored.
	if v := os.Getenv("JM_E2E_STALE_MS"); v != "" && v != "0" {
		fmt.Fprintln(os.Stderr, "JM_E2E_STALE_MS is no longer a setting: staleMs must be 0 (docs/adr/0003-pure-pod-core.md, item 5)")
		os.Exit(2)
	}
	if err := checkRemote(serverCmd, remoteURL, remoteBearer); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(2)
	}
	code := m.Run()
	if serverCmd != "" || remoteURL != "" {
		if code = judgeRan(code, stepsRun.Load()); code == 1 && stepsRun.Load() == 0 {
			fmt.Fprintln(os.Stderr, "no test ran against the server; a run that tests nothing is a failure")
		}
		fmt.Println(summaryLine(time.Since(suiteStart).Milliseconds(), stepsOK.Load(), stepsRun.Load()))
	}
	os.Exit(code)
}

// judgeRan turns a green run in which no test ran into a failure: a server that nothing tested has not passed.
func judgeRan(code int, ran int64) int {
	if ran == 0 && code == 0 {
		return 1
	}
	return code
}

// checkRemote refuses a remote setup that would test something other than what it names.
func checkRemote(cmd, url, bearer string) error {
	switch {
	case url == "":
		return nil
	case cmd != "":
		return fmt.Errorf("JM_E2E_URL and JM_E2E_SERVER are both set: the suite either boots a server or tests a deployed one")
	case !strings.HasPrefix(url, "https://") && !strings.HasPrefix(url, "http://127.0.0.1"):
		return fmt.Errorf("JM_E2E_URL must be https (or http on 127.0.0.1)")
	case bearer == "":
		return fmt.Errorf("JM_E2E_URL needs JM_E2E_TEST_BEARER: a deployed server's /__test routes need the bearer")
	}
	return nil
}

func need(t *testing.T) {
	t.Helper()
	if serverCmd == "" && remoteURL == "" {
		t.Skip("JM_E2E_SERVER and JM_E2E_URL are not set; run `make e2e` with a server command (docs/development/e2e-contract.md)")
	}
}

// needLocal skips tests that start, kill or inspect the server process; a deployed server is not ours to restart.
func needLocal(t *testing.T) {
	t.Helper()
	need(t)
	if remoteURL != "" {
		t.Skip("JM_E2E_URL is set: this test needs to kill and restart the server, or read its process")
	}
}

// needFake skips tests that drive the clock; real-clock mode never calls /__test/clock.
func needFake(t *testing.T) {
	t.Helper()
	need(t)
	if realClock {
		t.Skip("real-clock mode: this test needs the fake clock (/__test/clock)")
	}
}

// needClockFree is for a test that never touches the clock and never restarts the server, so it runs in every mode, remote included.
func needClockFree(t *testing.T) { t.Helper(); need(t) }

func track(t *testing.T) {
	countTest(t)
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
	remote     string // the deployed server's URL; no process, no port
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
	if remoteURL != "" {
		return &server{t: t, remote: remoteURL}
	}
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

func (s *server) base() string {
	if s.remote != "" {
		return s.remote
	}
	return fmt.Sprintf("http://127.0.0.1:%d", s.port)
}

// dial opens the pod WebSocket upgrade on the server, over TLS when the server is remote and https.
func (s *server) dial(path, token string) (net.Conn, *bufio.Reader, error) {
	if s.remote == "" {
		return wsDial(fmt.Sprintf("127.0.0.1:%d", s.port), path, token)
	}
	u, err := url.Parse(s.remote)
	if err != nil {
		return nil, nil, err
	}
	if u.Scheme != "https" {
		return wsDial(u.Host, path, token)
	}
	return wsDialTLS(u.Hostname()+":443", u.Hostname(), path, token)
}

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

type helloData struct {
	PodID   string `json:"podId"`
	HeadSeq *int64 `json:"headSeq"` // a pointer: a hello with no headSeq is not a hello with headSeq 0
	You     Actor  `json:"you"`
	Members []struct {
		Actor Actor
		Role  string
	} `json:"members"`
	Agents []struct{ Agent Actor } `json:"agents"`
}

// seedUser mints a user through the contract's seed route (POST /__test/session) and checks the jmu_ token.
func seedUser(c *client, name string) (user, error) {
	var out struct {
		Token string `json:"token"`
		User  Actor  `json:"user"`
	}
	if r := c.do("POST", "/__test/session", "", map[string]any{"name": name}, &out); !r.ok() {
		return user{}, fmt.Errorf("HTTP %d code=%q", r.Status, r.Err.Code)
	}
	if !strings.HasPrefix(out.Token, "jmu_") || out.User.ID == "" {
		return user{}, fmt.Errorf("token %q user %+v", out.Token, out.User)
	}
	return user{tok: out.Token, actor: out.User}, nil
}

// firstHello checks that the first frame s received is hello, and decodes it.
func firstHello(s *sock) (helloData, error) {
	var h helloData
	s.waitAfter(0, func(Frame) bool { return true }, 5*time.Second)
	first := s.snapshot(0)
	if len(first) == 0 || first[0].Type != "hello" {
		return h, fmt.Errorf("the first frame is %+v, want hello", first)
	}
	first[0].dec(&h)
	if h.HeadSeq == nil {
		return h, fmt.Errorf("hello carries no headSeq: %s", first[0].Data)
	}
	return h, nil
}
