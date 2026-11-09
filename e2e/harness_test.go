// Package e2e is the black-box contract suite: it runs unchanged against any server that implements
// docs/protocol.md plus the test-route contract in docs/development/e2e-contract.md.
package e2e

import (
	"bytes"
	"flag"
	"fmt"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"runtime"
	"slices"
	"strconv"
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
	realClock  = os.Getenv("JM_E2E_REAL_CLOCK") == "1"
	// stateInDatabase is set by `make e2e-c` when the server keeps its state in a real database (DATABASE_URL): the --persist-to
	// directory then holds no files by design. TestF02 says so aloud and skips only its directory-content check.
	stateInDatabase = os.Getenv("JM_E2E_STATE_IN_DB") == "1"
	suiteTests      = []string{"TestF02", "TestF03", "TestF04", "TestF05", "TestF06", "TestF07", "TestF08", "TestF09", "TestF10", "TestF11", "TestF12", "TestF13", "TestF14", "TestExpiryZeroTraffic", "TestRenewRace",
		"TestKillMidLease", "TestReplayAcrossRestart", "TestOneReleasePerCause", "TestInputValidation", "TestRestartPastDeadline", "TestRunTokenOtherPod", "TestEvictMidClaim", "TestEvictAlarmWake", "TestEvictPastDeadline", "TestF15", "TestBSESS", "TestAUTHH", "TestDENY"}
	outcomes sync.Map // test name -> failed bool
)

// suiteWG counts the parallel suite tests that have started and not yet finished (cleanups included); the budget test waits on it.
var suiteWG sync.WaitGroup

// par runs the test in parallel with the others. Every test that calls it owns its server, its data dir and its pod, so
// nothing is shared but the process: two tests that shared a pod would see each other's events. Call it right after need.
// The budget test waits for every test that called it, so it still judges the whole run.
func par(t *testing.T) {
	t.Helper()
	suiteWG.Add(1)
	t.Cleanup(suiteWG.Done) // registered first, so it runs last: after track's outcome and the server's kill
	noStrays(t)
	t.Parallel()
}

func TestMain(m *testing.M) {
	// staleMs is 0 (ADR 0003, "What an adapter must do", item 5): every command is persisted, heartbeats included.
	// The knob that once widened the early-expiry bound is gone, and a value other than 0 is an error, not ignored.
	if v := os.Getenv("JM_E2E_STALE_MS"); v != "" && v != "0" {
		fmt.Fprintln(os.Stderr, "JM_E2E_STALE_MS is no longer a setting: staleMs must be 0 (docs/adr/0003-pure-pod-core.md, item 5)")
		os.Exit(2)
	}
	flag.Parse()
	set := false
	flag.Visit(func(f *flag.Flag) { set = set || f.Name == "test.parallel" })
	if p := flag.Lookup("test.parallel"); p != nil {
		given, _ := strconv.Atoi(p.Value.String())
		_ = p.Value.Set(strconv.Itoa(suiteParallel(set, given, runtime.GOMAXPROCS(0))))
	}
	code := m.Run()
	if serverCmd != "" {
		fmt.Println(summaryLine(time.Since(suiteStart).Milliseconds(), stepsOK.Load(), stepsRun.Load()))
	}
	os.Exit(code)
}

// suiteParallel is the -parallel the suite runs with. TestF14Budget is parallel and only waits, but it holds a slot from the
// moment it resumes to the end of the run, so a run with one slot per CPU (go test's default) works one slot short for most
// of its length. Unset, the suite gets one slot per CPU plus that one. Set by hand (-parallel n), n is kept as given.
func suiteParallel(set bool, given, procs int) int {
	if set {
		return given
	}
	return procs + 1
}

func need(t *testing.T) {
	t.Helper()
	if serverCmd == "" {
		t.Skip("JM_E2E_SERVER is not set; run `make e2e` with a server command (docs/development/e2e-contract.md)")
	}
	// A server may skip the tests that need a route it cannot serve (the object routes of the Cloudflare adapter). Each
	// name is in the contract doc with its reason. A skipped test counts as passed for the budget: it is not a failure.
	if slices.Contains(strings.Split(os.Getenv("JM_E2E_SKIP"), ","), t.Name()) {
		outcomes.Store(t.Name(), false)
		t.Skip("named in JM_E2E_SKIP: this server has no route for it (docs/development/e2e-contract.md, \"Skipped on the server\")")
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

// lineCap bounds each line of a tail; a Worker's `stack:` line alone runs past 1 KB.
const lineCap = 500

var (
	failLine = regexp.MustCompile(`ERROR|Fatal|failed to start`) // a line that names a failure
	ansiCode = regexp.MustCompile(`\x1b\[[0-9;]*[A-Za-z]`)
)

// logLines is the output as lines, without colour codes, empty lines or `stack:` dumps, each cut at lineCap.
func logLines(b []byte) []string {
	var out []string
	for _, ln := range strings.Split(ansiCode.ReplaceAllString(string(b), ""), "\n") {
		ln = strings.TrimRight(ln, "\r")
		if strings.TrimSpace(ln) == "" || strings.HasPrefix(strings.TrimSpace(ln), "stack:") {
			continue
		}
		if len(ln) > lineCap {
			ln = ln[:lineCap] + "..."
		}
		out = append(out, ln)
	}
	return out
}

// tail is at most n lines: those that name a failure, from anywhere in the log, then the last of the rest. A cause printed
// at second 2 still shows after a minute of other output.
func (l *logBuf) tail(n int) string {
	var fail, rest []string
	for _, ln := range logLines(l.bytes()) {
		if failLine.MatchString(ln) {
			fail = append(fail, ln)
		} else {
			rest = append(rest, ln)
		}
	}
	fail = fail[:min(n, len(fail))]
	return strings.Join(append(fail, rest[max(0, len(rest)-(n-len(fail))):]...), "\n")
}

func (l *logBuf) bytes() []byte {
	l.mu.Lock()
	defer l.mu.Unlock()
	return slices.Clone(l.b.Bytes())
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
	env        []string      // extra NAME=value pairs for this server only (TestF19's vault key); every start gets them
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

// portTries bounds the first start of a server. freePort closes its listener before the server binds, so
// another process can take the port in between. That failure, and a Workers runtime that failed to start, are retried on fresh ports.
const portTries = 3

// portTaken matches the output of a start that exited because its port was taken.
var portTaken = regexp.MustCompile(`(?i)address already in use|EADDRINUSE`)

// runtimeFailed matches a Worker start whose runtime (workerd) died before serving. The cause is not known yet; the
// retry line carries the first line that names a failure, so the next one that happens says what it was.
var runtimeFailed = regexp.MustCompile(`Workers runtime failed to start`)

// startRetry says whether a failed start on `port` is tried again on fresh ports, and the line to print for try `try`.
func startRetry(name string, port, try int, out []byte) (string, bool) {
	at := fmt.Sprintf("test=%s port=%d try=%d", name, port, try)
	if portTaken.Match(out) {
		return "PORT_RETRY " + at, true
	}
	if !runtimeFailed.Match(out) {
		return "", false
	}
	for _, ln := range logLines(out) {
		if failLine.MatchString(ln) {
			return "START_RETRY " + at + " cause=" + strings.TrimSpace(ln), true
		}
	}
	return "START_RETRY " + at, true
}

func startServer(t *testing.T, env ...string) *server {
	port := freePort(t)
	s := &server{t: t, port: port, inspector: freePort(t, port), dir: t.TempDir(), log: &logBuf{}, env: env}
	t.Cleanup(func() {
		s.kill()
		if t.Failed() {
			t.Logf("server log tail:\n%s", s.log.tail(40))
		}
	})
	for try := 1; !s.start(); try++ {
		line, retry := startRetry(t.Name(), s.port, try+1, s.log.bytes())
		if try == portTries || !retry {
			s.t.Fatalf("server exited before /healthz answered 200; log tail:\n%s", s.log.tail(20))
		}
		s.kill()
		fmt.Println(line)
		s.port = freePort(t, s.port, s.inspector)
		s.inspector = freePort(t, s.port)
		s.dir, s.log = t.TempDir(), &logBuf{}
	}
	return s
}

func (s *server) base() string { return fmt.Sprintf("http://127.0.0.1:%d", s.port) }

func (s *server) boot() {
	if !s.start() {
		s.t.Fatalf("server exited before /healthz answered 200; log tail:\n%s", s.log.tail(20))
	}
}

// start runs the server command and waits for /healthz. It returns false if the process exited first.
func (s *server) start() bool {
	begin := time.Now()
	root, _ := filepath.Abs("..")
	cmd := exec.Command("sh", "-c", serverCmd)
	cmd.Dir = root
	cmd.Env = append(os.Environ(), fmt.Sprintf("PORT=%d", s.port), fmt.Sprintf("INSPECTOR_PORT=%d", s.inspector), "ENV=local", "JM_DATA_DIR="+s.dir)
	cmd.Env = append(cmd.Env, s.env...)
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	cmd.Stdout, cmd.Stderr = s.log, s.log
	if err := cmd.Start(); err != nil {
		s.t.Fatalf("cannot start JM_E2E_SERVER: %v", err)
	}
	s.cmd, s.exited = cmd, make(chan struct{})
	go func(c *exec.Cmd, ch chan struct{}) { _ = c.Wait(); close(ch) }(cmd, s.exited)
	if !s.waitHealthy() {
		return false
	}
	if s.bootTook == 0 {
		s.bootTook = time.Since(begin)
	}
	return true
}

func (s *server) waitHealthy() bool {
	hc := &http.Client{Timeout: 2 * time.Second}
	limit := 40 * time.Second
	if s.restarting {
		limit = 10 * time.Second
	}
	deadline := time.Now().Add(limit)
	for time.Now().Before(deadline) {
		select {
		case <-s.exited:
			return false
		default:
		}
		if resp, err := hc.Get(s.base() + "/healthz"); err == nil {
			resp.Body.Close()
			if resp.StatusCode == 200 {
				return true
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
	return false
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
