package e2e

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"slices"
	"strings"
	"syscall"
	"testing"
	"time"
)

// The daemon under test is the repo's own jellymoon binary, built once into a temp dir and run as a black box: its argv, its
// environment, its config dir (JELLYMOON_HOME) and its output. Its files are documented in docs/development/e2e-contract.md.

// dproc is one daemon or CLI process in its own process group, with its output captured.
type dproc struct {
	cmd  *exec.Cmd
	out  *logBuf
	done chan struct{}
}

func spawn(t *testing.T, bin string, env []string, args ...string) *dproc {
	t.Helper()
	p := &dproc{cmd: exec.Command(bin, args...), out: &logBuf{}, done: make(chan struct{})}
	p.cmd.Env = env
	p.cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	p.cmd.Stdout, p.cmd.Stderr = p.out, p.out
	if err := p.cmd.Start(); err != nil {
		t.Fatalf("cannot start %s %v: %v", bin, args, err)
	}
	go func() { _ = p.cmd.Wait(); close(p.done) }()
	t.Cleanup(p.kill)
	return p
}

func (p *dproc) signal(sig syscall.Signal) { _ = syscall.Kill(-p.cmd.Process.Pid, sig) }

func (p *dproc) kill() {
	p.signal(syscall.SIGKILL)
	<-p.done
}

func (p *dproc) text() string {
	p.out.mu.Lock()
	defer p.out.mu.Unlock()
	return p.out.b.String()
}

// finish waits for the process to exit by itself.
func (p *dproc) finish(d time.Duration) (int, bool) {
	select {
	case <-p.done:
		return p.cmd.ProcessState.ExitCode(), true
	case <-time.After(d):
		return 0, false
	}
}

type storedComputer struct {
	Server     string `json:"server"`
	ComputerID string `json:"computerId"`
	Token      string `json:"token"`
}

type presenceData struct {
	Actor      Actor  `json:"actor"`
	State      string `json:"state"`
	ComputerID string `json:"computerId"`
}

func presenceOf(agentID, state string) func(Frame) bool {
	return func(f Frame) bool {
		var d presenceData
		f.dec(&d)
		return f.Type == "presence.changed" && d.Actor.ID == agentID && d.State == state
	}
}

// helloAgents dials as tok and returns the agents hello lists, with their states.
func helloAgents(t *testing.T, srv *server, pod, tok string) map[string]string {
	t.Helper()
	conn, br, err := wsDial(fmt.Sprintf("127.0.0.1:%d", srv.port), "/v1/pods/"+pod+"/ws", tok)
	if err != nil {
		t.Fatalf("hello dial: %v", err)
	}
	s := newSock("hello", pod, conn, br)
	defer s.close()
	f, ok := s.waitAfter(0, isType("hello"), 5*time.Second)
	if !ok {
		t.Fatalf("no hello within 5 s of the upgrade")
	}
	var h struct {
		Agents []struct {
			Agent Actor
			State string
		}
	}
	f.dec(&h)
	out := map[string]string{}
	for _, a := range h.Agents {
		out[a.Agent.ID] = a.State
	}
	return out
}

func openAs(t *testing.T, srv *server, pod string, u user) *sock {
	t.Helper()
	conn, br, err := wsDial(fmt.Sprintf("127.0.0.1:%d", srv.port), "/v1/pods/"+pod+"/ws", u.tok)
	if err != nil {
		t.Fatalf("%s: WebSocket: %v", u.actor.DisplayName, err)
	}
	s := newSock(u.actor.DisplayName, pod, conn, br)
	t.Cleanup(s.close)
	if _, ok := s.waitAfter(0, isType("hello"), 5*time.Second); !ok {
		t.Fatalf("%s: no hello", u.actor.DisplayName)
	}
	s.send("subscribe", map[string]any{"sinceSeq": 0})
	return s
}

func buildDaemon(t *testing.T) string {
	t.Helper()
	root, _ := filepath.Abs("..")
	bin := filepath.Join(t.TempDir(), "jellymoon")
	cmd := exec.Command("go", "build", "-o", bin, "./daemon/cmd/jellymoon")
	cmd.Dir = root
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("go build ./daemon/cmd/jellymoon: %v\n%s", err, out)
	}
	return bin
}

func readComputer(cfg string) (storedComputer, os.FileInfo, error) {
	var sc storedComputer
	path := filepath.Join(cfg, "computer.json")
	fi, err := os.Stat(path)
	if err != nil {
		return sc, nil, err
	}
	raw, err := os.ReadFile(path)
	if err == nil {
		err = json.Unmarshal(raw, &sc)
	}
	return sc, fi, err
}

// waitComputer waits for the daemon to have stored its computer.
func waitComputer(cfg string) (sc storedComputer, fi os.FileInfo, err error) {
	for end := time.Now().Add(10 * time.Second); time.Now().Before(end); time.Sleep(25 * time.Millisecond) {
		if sc, fi, err = readComputer(cfg); err == nil && sc.Token != "" {
			return
		}
	}
	return
}

var secretShape = regexp.MustCompile(`jm[ucr]_[A-Za-z0-9]{10}`)

// leaks lists every place a token shows: the captured output of each process, and the argv of every process on the host.
func leaks(procs map[string]*dproc, secrets ...string) []string {
	var hits []string
	check := func(where, text string) {
		for _, s := range secrets {
			if strings.Contains(text, s) {
				hits = append(hits, where+" holds a full token")
			}
		}
		if m := secretShape.FindString(text); m != "" {
			hits = append(hits, where+" holds a token-shaped string "+m)
		}
	}
	for name, p := range procs {
		check(name+" output", p.text())
	}
	argv, _ := exec.Command("ps", "-ww", "-axo", "args=").Output()
	check("ps argv", string(argv))
	return hits
}

// f08 is the test's state: one server, a pod with two members watching it, and the daemon's config dir.
type f08 struct {
	t       *testing.T
	srv     *server
	c       *client
	bin     string
	cfg     string
	pod     string
	clock   int64
	owner   user
	ben     user
	obs     []*sock
	env     []string
	agentID string
	comp    storedComputer
	daemon  *dproc
	procs   map[string]*dproc
}

func newF08(t *testing.T) *f08 {
	f := &f08{t: t, bin: buildDaemon(t), srv: startServer(t), clock: fakeBase, procs: map[string]*dproc{}}
	f.c = newClient(t, f.srv.base())
	f.cfg = filepath.Join(t.TempDir(), "jm") // the daemon creates it
	f.env = []string{"JELLYMOON_HOME=" + f.cfg, "JELLYMOON_SERVER=" + f.srv.base()}
	f.setClock(f.clock)
	f.owner, f.ben = mustUser(t, f.c, "olivia"), mustUser(t, f.c, "ben")
	var pod, inv struct{ ID, Code string }
	if r := f.c.do("POST", "/v1/pods", f.owner.tok, map[string]any{"name": "presence"}, &pod); !r.ok() {
		t.Fatalf("POST /v1/pods: HTTP %d", r.Status)
	}
	f.pod = pod.ID
	f.c.do("POST", "/v1/pods/"+f.pod+"/invites", f.owner.tok, map[string]any{"maxUses": 1}, &inv)
	f.c.do("POST", "/v1/invites/"+inv.Code+"/accept", f.ben.tok, nil, nil)
	f.obs = []*sock{openAs(t, f.srv, f.pod, f.owner), openAs(t, f.srv, f.pod, f.ben)}
	return f
}

func (f *f08) setClock(now int64) {
	f.clock = now
	f.c.do("POST", "/__test/clock", "", map[string]any{"now": now}, nil)
}

func (f *f08) marks() (m []int) {
	for _, s := range f.obs {
		m = append(m, s.mark())
	}
	return
}

// all waits for every observer to see a presence.changed for the agent in `state` after its mark.
func (f *f08) all(state string, marks []int, d time.Duration) error {
	for i, s := range f.obs {
		if _, ok := s.waitAfter(marks[i], presenceOf(f.agentID, state), d); !ok {
			return fmt.Errorf("%s saw no presence.changed %s for the agent within %v", s.name, state, d)
		}
	}
	return nil
}

func (f *f08) startDaemon(name string, extra ...string) {
	f.daemon = spawn(f.t, f.bin, append(slices.Clone(f.env), extra...), "daemon", "start", "--foreground")
	f.procs[name] = f.daemon
}

func (f *f08) registersComputer() error {
	f.startDaemon("daemon", "JELLYMOON_USER_TOKEN="+f.owner.tok)
	comp, fi, err := waitComputer(f.cfg)
	if err != nil {
		return fmt.Errorf("no stored computer in %s within 10 s: %v\ndaemon output:\n%s", f.cfg, err, f.daemon.text())
	}
	f.comp = comp
	dir, _ := os.Stat(f.cfg)
	if fi.Mode().Perm() != 0o600 || dir.Mode().Perm() != 0o700 {
		return fmt.Errorf("computer.json is %v and the config dir %v, want 0600 and 0700", fi.Mode().Perm(), dir.Mode().Perm())
	}
	if len(comp.Token) != 47 || !strings.HasPrefix(comp.Token, "jmc_") || !strings.HasPrefix(comp.ComputerID, "cmp_") {
		return fmt.Errorf("stored computer %q token %d chars, want cmp_ and a 47-character jmc_ token", comp.ComputerID, len(comp.Token))
	}
	return nil
}

func (f *f08) addsAgent() error {
	cli := spawn(f.t, f.bin, f.env, "agent", "add", "fake", "--pod", f.pod, "--name", "fake-one")
	f.procs["cli"] = cli
	if code, ok := cli.finish(10 * time.Second); !ok || code != 0 {
		return fmt.Errorf("agent add exited %d (finished=%v):\n%s", code, ok, cli.text())
	}
	if f.agentID = regexp.MustCompile(`agt_[0-9A-Z]{26}`).FindString(cli.text()); f.agentID == "" {
		return fmt.Errorf("agent add printed no agt_ id:\n%s", cli.text())
	}
	return nil
}

func (f *f08) showsOnline() error {
	// The observers have been subscribed since before agent add, so mark 0 is the start of their frames.
	if err := f.all("online", []int{0, 0}, 5*time.Second); err != nil {
		return err
	}
	if st := helloAgents(f.t, f.srv, f.pod, f.ben.tok)[f.agentID]; st != "online" {
		return fmt.Errorf("hello lists the agent as %q, want online", st)
	}
	return nil
}

// hangs stops the daemon with its socket open, then moves the server clock to exactly 45 s after its last sign of life.
func (f *f08) hangs() error {
	marks := f.marks()
	f.daemon.signal(syscall.SIGSTOP)
	time.Sleep(300 * time.Millisecond)
	f.setClock(f.clock + 44999)
	if _, got := f.obs[0].waitAfter(marks[0], presenceOf(f.agentID, "offline"), 400*time.Millisecond); got {
		return fmt.Errorf("the agent went offline 44.999 s after the last heartbeat; the bound is 45 s")
	}
	f.setClock(f.clock + 1)
	return f.all("offline", marks, 5*time.Second)
}

func (f *f08) restartsOnSameComputer() error {
	f.daemon.kill()
	before, _, _ := readComputer(f.cfg)
	marks := f.marks()
	f.startDaemon("daemon2")
	if err := f.all("online", marks, 5*time.Second); err != nil {
		return fmt.Errorf("%v\ndaemon output:\n%s", err, f.daemon.text())
	}
	if after, _, _ := readComputer(f.cfg); after != before {
		return fmt.Errorf("the second start changed the stored computer: %+v then %+v", before, after)
	}
	return nil
}

func (f *f08) goesOfflineWhenKilled() error {
	marks := f.marks()
	f.daemon.kill()
	return f.all("offline", marks, 45*time.Second)
}

func (f *f08) leaksNothing() error {
	if hits := leaks(f.procs, f.owner.tok, f.comp.Token); len(hits) > 0 {
		return fmt.Errorf("%s", strings.Join(hits, "; "))
	}
	return nil
}

func (f *f08) refusesPlainHTTP() error {
	env := []string{"JELLYMOON_HOME=" + f.t.TempDir(), "JELLYMOON_SERVER=http://example.invalid", "JELLYMOON_USER_TOKEN=" + f.owner.tok}
	bad := spawn(f.t, f.bin, env, "daemon", "start", "--foreground")
	code, ok := bad.finish(5 * time.Second)
	if !ok || code == 0 || !strings.Contains(bad.text(), "https") || strings.Contains(bad.text(), f.owner.tok) {
		return fmt.Errorf("exit %d finished=%v; output wanted to name the https rule and no token:\n%s", code, ok, bad.text())
	}
	return nil
}

// restoredPodIsHonest crashes the server while the daemon is online and the daemon is gone when it returns.
func (f *f08) restoredPodIsHonest() error {
	marks := f.marks()
	f.startDaemon("daemon3")
	if err := f.all("online", marks, 5*time.Second); err != nil {
		return err
	}
	f.srv.kill()
	f.daemon.kill()
	f.srv.restarting = true
	f.srv.boot()
	f.srv.restarting = false
	if st := helloAgents(f.t, f.srv, f.pod, f.ben.tok)[f.agentID]; st != "offline" {
		return fmt.Errorf("after a server crash with the daemon gone, hello lists the agent as %q, want offline", st)
	}
	return nil
}

// TestF08: the daemon registers its computer and an agent, and the pod shows the agent online and then offline.
func TestF08(t *testing.T) {
	needFake(t)
	track(t)
	f := newF08(t)
	p := &stepper{out: os.Stdout, events: func() []Frame { return f.obs[0].snapshot(0) }}
	defer p.count(t)
	p.step("jellymoon daemon start --foreground registers the computer and stores jmc_ at mode 0600", f.registersComputer)
	p.step("jellymoon agent add fake registers an agent in the pod", f.addsAgent)
	p.step("members receive presence.changed online for the agent within 5 s", f.showsOnline)
	p.step("a daemon that hangs with its socket open goes offline 45 s after its last sign of life, on the server clock", f.hangs)
	p.step("a second daemon start reuses the stored computer and the agent comes back online", f.restartsOnSameComputer)
	p.step("killing the daemon produces presence.changed offline within 45 s", f.goesOfflineWhenKilled)
	p.step("no token reaches any output or argv", f.leaksNothing)
	p.step("the daemon refuses plain http to a host that is not local before it sends a token", f.refusesPlainHTTP)
	p.step("a restored pod does not report a computer whose socket is gone as online", f.restoredPodIsHonest)
	if p.ok != p.run {
		t.Fatalf("%d of %d steps passed", p.ok, p.run)
	}
}

func mustUser(t *testing.T, c *client, name string) user {
	t.Helper()
	u, err := seedUser(c, name)
	if err != nil {
		t.Fatalf("POST /__test/session: %v", err)
	}
	return u
}
