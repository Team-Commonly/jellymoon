package e2e

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"
	"syscall"
	"testing"
	"time"
)

// TestF22: a pod owner connects, grants and names approvers with `jellymoon connector ...` (feature_list.json F22). The CLI is
// the real binary; the server is the real server behind a recording proxy; the upstream is F19's stub Linear.

// f22 is an F19 world plus the built CLI, one config dir and the record of everything the CLI was given or printed.
type f22 struct {
	*f19
	bin, home string
	rec       *recorder
	argv      [][]string // every command line
	streams   []string   // every stdout and stderr
	released  string     // the decision of a write a listed member approved
	strays    []string   // values passed on argv on purpose, to see they are never echoed
}

type cliRun struct {
	out, errOut string
	code        int
}

var conID = regexp.MustCompile(`con_[0-9A-HJKMNP-TV-Z]{26}`)

// cli runs `jellymoon connector <args>` as `as`, with extra environment. It has no controlling terminal, so the hidden prompt
// cannot open: a missing credential must fail, not wait.
func (f *f22) cli(as user, env []string, args ...string) cliRun {
	cmd := exec.Command(f.bin, append([]string{"connector"}, args...)...)
	cmd.Dir = f.t.TempDir()
	cmd.Env = append([]string{"PATH=" + os.Getenv("PATH"), "JELLYMOON_HOME=" + f.home, "JELLYMOON_SERVER=" + f.rec.URL, "JELLYMOON_USER_TOKEN=" + as.tok}, env...)
	cmd.SysProcAttr = &syscall.SysProcAttr{Setsid: true}
	var so, se bytes.Buffer
	cmd.Stdout, cmd.Stderr = &so, &se
	code := 0
	if err := cmd.Run(); err != nil {
		ee, ok := err.(*exec.ExitError)
		if !ok {
			f.t.Fatalf("running the CLI: %v", err)
		}
		code = ee.ExitCode()
	}
	f.argv = append(f.argv, cmd.Args)
	f.streams = append(f.streams, so.String(), se.String())
	return cliRun{so.String(), se.String(), code}
}

func (f *f22) tokenEnv() []string { return []string{"JELLYMOON_CONNECTOR_TOKEN=" + stubToken} }

// refused checks a run failed with a non-zero exit and the server's code in its stderr, and printed nothing on stdout.
func refused(what string, r cliRun, codes ...string) error {
	if r.code == 0 || r.out != "" {
		return fmt.Errorf("%s: exit %d, stdout %q, want a non-zero exit and nothing on stdout", what, r.code, r.out)
	}
	for _, c := range codes {
		if !strings.Contains(r.errOut, c) {
			return fmt.Errorf("%s: stderr %q lacks %q", what, r.errOut, c)
		}
	}
	return nil
}

func (f *f22) hits(method, path string) int {
	n := 0
	for _, h := range f.rec.hitsAll() {
		if h.Method == method && h.Path == f.podPath(path) {
			n++
		}
	}
	return n
}

func (f *f22) connectArgs(url string, extra ...string) []string {
	return append([]string{"connect", "linear", "--pod", f.pod, "--url", url}, extra...)
}

// Step 1: connect. A member is forbidden, a drifted upstream is refused naming the operation, a missing credential fails before
// any request, a credential on argv is refused unechoed, and the owner connects.
func (f *f22) connects() error {
	before := f.hits("POST", "connections")
	if err := refused("a member's connect", f.cli(f.ben, f.tokenEnv(), f.connectArgs(f.linear.srv.URL)...), "forbidden"); err != nil {
		return err
	}
	drifted := newStubLinear(f.t, "create_issue")
	if err := refused("a connect to a drifted upstream", f.cli(f.owner, f.tokenEnv(), f.connectArgs(drifted.srv.URL)...), "connector_drift", "create_issue"); err != nil {
		return err
	}
	if got := f.hits("POST", "connections") - before; got != 2 {
		return fmt.Errorf("the refused connects made %d requests, want 2", got)
	}
	posts := f.hits("POST", "connections")
	if err := refused("a connect with no credential and no terminal", f.cli(f.owner, nil, f.connectArgs(f.linear.srv.URL)...), "JELLYMOON_CONNECTOR_TOKEN"); err != nil {
		return err
	}
	stray := "stray" + "-argument-" + "value"
	f.strays = append(f.strays, stray)
	for what, args := range map[string][]string{"a --token flag": f.connectArgs(f.linear.srv.URL, "--token", stray), "a credential as an argument": f.connectArgs(f.linear.srv.URL, stray)} {
		r := f.cli(f.owner, f.tokenEnv(), args...)
		if err := refused(what, r); err != nil || strings.Contains(r.errOut, stray) {
			return fmt.Errorf("%s: %v (stderr %q must not echo the argument)", what, err, r.errOut)
		}
	}
	if f.hits("POST", "connections") != posts {
		return fmt.Errorf("a refused command line reached the server")
	}
	r := f.cli(f.owner, f.tokenEnv(), f.connectArgs(f.linear.srv.URL)...)
	f.conn = conID.FindString(r.out)
	if r.code != 0 || f.conn == "" || !strings.Contains(r.out, "linear") || !strings.Contains(r.out, "list_issues") || !strings.Contains(r.out, "create_issue") {
		return fmt.Errorf("the owner's connect: exit %d, stdout %q stderr %q, want a con_ id, the vendor and the pinned operations", r.code, r.out, r.errOut)
	}
	return nil
}

// Step 2: grant. `--access write` never leaves the client; a member and an unknown agent are refused; the grants work for the
// agent: a read reads, a write is parked.
func (f *f22) grants() error {
	g := func(access string, agent string) []string {
		return []string{"grant", "--pod", f.pod, "--connection", f.conn, "--agent", agent, "--access", access}
	}
	n := f.hits("POST", "grants")
	for _, bad := range []string{"write", "admin", ""} {
		if err := refused("--access "+bad, f.cli(f.owner, nil, g(bad, f.a1.id)...), "write-with-confirm"); err != nil {
			return err
		}
	}
	if f.hits("POST", "grants") != n {
		return fmt.Errorf("a refused access reached the server")
	}
	if err := refused("a member's grant", f.cli(f.ben, nil, g("read", f.a1.id)...), "forbidden"); err != nil {
		return err
	}
	if err := refused("a grant to an agent not in the pod", f.cli(f.owner, nil, g("read", "agt_00000000000000000000000000")...), "not_found"); err != nil {
		return err
	}
	if r := f.cli(f.owner, nil, g("read", f.a1.id)...); r.code != 0 || !strings.Contains(r.out, f.a1.id) {
		return fmt.Errorf("granting read: exit %d, stdout %q stderr %q", r.code, r.out, r.errOut)
	}
	if out, isErr, b := f.connector("list_issues", map[string]any{}); isErr || out["state"] != "done" {
		return fmt.Errorf("a read on the CLI's read grant: %s, want state done", b)
	}
	if _, isErr, b := f.connector("create_issue", map[string]any{"title": "x", "teamId": "T"}); !isErr || !strings.Contains(string(b), "forbidden") {
		return fmt.Errorf("a write on the CLI's read grant: %s, want forbidden", b)
	}
	if r := f.cli(f.owner, nil, g("write-with-confirm", f.a1.id)...); r.code != 0 {
		return fmt.Errorf("granting write-with-confirm: exit %d, stderr %q", r.code, r.errOut)
	}
	dec, _, err := f.park(map[string]any{"title": "Parked by the CLI's grant", "teamId": "TEAM-1"})
	if err != nil {
		return err
	}
	if err := f.rule(f.sO, dec, "b", "ruled"); err != nil {
		return err
	}
	return f.writes(0)
}

func (f *f22) approve(as user, users string) cliRun {
	return f.cli(as, nil, "approvers", "--pod", f.pod, "--connection", f.conn, "--users", users)
}

// Step 3: approvers. A member is forbidden, a stranger is not found, leaving --users out is an error, naming ben lets him approve,
// and an empty list clears the set so he is refused again.
func (f *f22) approvers() error {
	if err := refused("a member naming approvers", f.approve(f.ben, f.ben.actor.ID), "forbidden"); err != nil {
		return err
	}
	if err := refused("a stranger as approver", f.approve(f.owner, "usr_00000000000000000000000000"), "not_found"); err != nil {
		return err
	}
	n := f.hits("POST", "approvers")
	if err := refused("approvers without --users", f.cli(f.owner, nil, "approvers", "--pod", f.pod, "--connection", f.conn), "--users"); err != nil || f.hits("POST", "approvers") != n {
		return fmt.Errorf("approvers without --users: %v (a request was made: %v)", err, f.hits("POST", "approvers") != n)
	}
	if r := f.approve(f.owner, f.ben.actor.ID); r.code != 0 || !strings.Contains(r.out, f.ben.actor.ID) {
		return fmt.Errorf("naming ben: exit %d, stdout %q stderr %q", r.code, r.out, r.errOut)
	}
	listed, _, err := f.park(map[string]any{"title": "Approved by a member the CLI named", "teamId": "TEAM-1"})
	if err != nil {
		return err
	}
	if err := f.rule(f.sCara, listed, "a", "forbidden"); err != nil {
		return err
	}
	if err := f.rule(f.sBen, listed, "a", "ruled"); err != nil {
		return err
	}
	if err := f.writesSoon(1); err != nil {
		return err
	}
	f.released = listed
	if r := f.approve(f.owner, ""); r.code != 0 || strings.Contains(r.out, f.ben.actor.ID) || !strings.Contains(r.out, "cleared") {
		return fmt.Errorf("clearing the set: exit %d, stdout %q stderr %q, want cleared and no ben", r.code, r.out, r.errOut)
	}
	removed, _, err := f.park(map[string]any{"title": "Ruled on after the set was cleared", "teamId": "TEAM-1"})
	if err != nil {
		return err
	}
	if err := f.rule(f.sBen, removed, "a", "forbidden"); err != nil {
		return err
	}
	if err := f.rule(f.sO, removed, "b", "ruled"); err != nil {
		return err
	}
	return f.writes(1)
}

// Step 4: list shows the pinned operations by access level, receipts shows the server's records.
func (f *f22) listsAndReceipts() error {
	for _, as := range []user{f.owner, f.ben} {
		r := f.cli(as, nil, "list", "--pod", f.pod)
		if r.code != 0 || !strings.Contains(r.out, f.conn) || !strings.Contains(r.out, "linear") || !regexp.MustCompile(`(?m)read.*list_issues.*get_issue`).MatchString(r.out) || !regexp.MustCompile(`(?m)write-with-confirm.*create_issue`).MatchString(r.out) {
			return fmt.Errorf("connector list: exit %d, stdout %q stderr %q, want the connection, the vendor and the operations by access level", r.code, r.out, r.errOut)
		}
	}
	var line string
	for end := time.Now().Add(5 * time.Second); time.Now().Before(end) && !strings.Contains(line, "upstream=200"); time.Sleep(25 * time.Millisecond) {
		line = f.cli(f.owner, nil, "receipts", "--pod", f.pod, "--decision", f.released).out
	}
	for _, want := range []string{"crc_", "released", "create_issue", "decision=" + f.released, "agent=" + f.a1.id, f.ben.actor.ID, "upstream=200"} {
		if !strings.Contains(line, want) {
			return fmt.Errorf("the receipt of the released write: %q lacks %q", line, want)
		}
	}
	all := strings.Split(strings.TrimSpace(f.cli(f.owner, nil, "receipts", "--pod", f.pod).out), "\n")
	one := strings.Split(strings.TrimSpace(f.cli(f.owner, nil, "receipts", "--pod", f.pod, "--limit", "1").out), "\n")
	if len(all) < 4 || len(one) != 1 || !strings.Contains(all[0], "crc_") {
		return fmt.Errorf("receipts: %d lines, and %d with --limit 1; want at least 4 and exactly 1", len(all), len(one))
	}
	return nil
}

// Step 5: the credential is in no argv, no output stream, no file in the config dir, no server log and no request but the connect.
func (f *f22) noToken() error {
	for _, a := range f.argv {
		if s := strings.Join(a, " "); strings.Contains(s, stubToken) || leakIn([]byte(s)) != "" {
			return fmt.Errorf("the credential is on a command line: %q", a)
		}
	}
	for _, s := range f.streams {
		if form := leakIn([]byte(s)); form != "" {
			return fmt.Errorf("the credential (%s) is in what the CLI printed: %.300s", form, s)
		}
		for _, stray := range f.strays {
			if strings.Contains(s, stray) {
				return fmt.Errorf("the CLI echoed an argument it refused")
			}
		}
	}
	files := 0
	_ = filepath.Walk(f.home, func(p string, fi os.FileInfo, err error) error {
		if err == nil && fi.Mode().IsRegular() {
			files++
			b, _ := os.ReadFile(p)
			if leakIn(b) != "" {
				f.t.Errorf("the credential is in %s", p)
			}
		}
		return nil
	})
	if files != 0 {
		return fmt.Errorf("the connector commands wrote %d files to the config dir, want none", files)
	}
	carried := 0
	for _, h := range f.rec.hitsAll() {
		if leakIn(h.Out) != "" || leakIn([]byte(h.Path+"?"+h.Query+" "+fmt.Sprint(h.Head))) != "" {
			return fmt.Errorf("the credential is in the answer to, or the path, query or a header of, %s %s", h.Method, h.Path)
		}
		if leakIn(h.In) != "" {
			if h.Method != "POST" || h.Path != f.podPath("connections") {
				return fmt.Errorf("the credential was sent with %s %s (HTTP %d), want only connect requests", h.Method, h.Path, h.Status)
			}
			carried++
		}
	}
	// The member's refused connect, the drifted one and the owner's: the three that passed the client's checks.
	if carried != 3 {
		return fmt.Errorf("the credential was in %d request bodies, want the 3 connects that reached the server", carried)
	}
	if form := leakIn(f.srv.log.bytes()); form != "" {
		return fmt.Errorf("the credential (%s) is in the server log", form)
	}
	return nil
}

func TestF22(t *testing.T) {
	needFake(t)
	par(t)
	f := &f22{f19: &f19{world: newWorldAt(t, fakeBase, "JM_VAULT_KEY="+f19VaultKey), linear: newStubLinear(t, ""), seen: map[string][]byte{}}}
	f.bin, f.home = buildDaemon(t), filepath.Join(t.TempDir(), "jm")
	f.rec = newRecorder(t, f.srv.base())
	f.r = f.startRun(f.a1, "triage the login bug")
	p := &stepper{out: os.Stdout, events: func() []Frame { return f.sO.snapshot(0) }}
	defer p.count(t)
	steps := []struct {
		name string
		fn   func() error
	}{
		{"connect reads the token from the environment, never argv; a member is forbidden and a drifted upstream is refused naming the operation", f.connects},
		{"grant takes read or write-with-confirm and refuses --access write before any request", f.grants},
		{"approvers replaces the set and an empty list clears it", f.approvers},
		{"list shows the pinned operations by access level and receipts shows the server's records", f.listsAndReceipts},
		{"the token is in no argv, stream, config file, server log or request but the connect", f.noToken},
	}
	for _, s := range steps {
		p.step(s.name, s.fn)
	}
	if p.ok != p.run {
		t.Fatalf("%d of %d steps passed", p.ok, p.run)
	}
}
