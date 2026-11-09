package e2e

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"syscall"
	"testing"
	"time"
)

// permRec is the part of the fake agent's record these cases read (e2e/testdata/fakeagent).
type permRec struct {
	PID        int
	Prompt     json.RawMessage
	Permission json.RawMessage
	Cancelled  bool
	Signalled  bool
}

// wait waits for the fake agent's record to satisfy ok.
func (f *f09) wait(path, what string, ok func(permRec) bool) (permRec, error) {
	var r permRec
	for end := time.Now().Add(15 * time.Second); time.Now().Before(end); time.Sleep(25 * time.Millisecond) {
		if raw, err := os.ReadFile(path); err == nil && json.Unmarshal(raw, &r) == nil && ok(r) {
			return r, nil
		}
	}
	return r, fmt.Errorf("the fake agent never reached %s within 15 s: %+v\ndaemon output:\n%s", what, r, f.daemon.text())
}

// wakes registers the computer, adds the fake agent (with the allow setting or not), waits until it is online and assigns it a task.
// It returns the claimed run.
func (f *f09) wakes(allow bool) error {
	if err := f.registersComputer(); err != nil {
		return err
	}
	args := []string{"agent", "add", "fake", "--pod", f.pod, "--name", "fake-one"}
	if allow {
		args = append(args, "--allow-permissions")
	}
	cli := spawn(f.t, f.bin, f.env, args...)
	f.procs["cli"] = cli
	if code, ok := cli.finish(10 * time.Second); !ok || code != 0 {
		return fmt.Errorf("agent add exited %d (finished=%v):\n%s", code, ok, cli.text())
	}
	if f.agentID = regexp.MustCompile(`agt_[0-9A-Z]{26}`).FindString(cli.text()); f.agentID == "" {
		return fmt.Errorf("agent add printed no agt_ id:\n%s", cli.text())
	}
	if err := f.all("online", []int{0, 0}, 5*time.Second); err != nil {
		return err
	}
	f.assign("ask for permission")
	h, ok := f.px.find("POST", runPath, 10*time.Second)
	if !ok {
		return fmt.Errorf("the daemon never called POST /v1/runs\ndaemon output:\n%s", f.daemon.text())
	}
	return json.Unmarshal(h.Out, &f.run)
}

// endedAs checks the pod saw the daemon end the run with this outcome.
func (f *f09) endedAs(outcome string) error {
	h, ok := f.px.find("POST", regexp.MustCompile(`^/v1/runs/`+f.run.RunID+`/end$`), 10*time.Second)
	if !ok || !bytes.Contains(h.In, []byte(`"`+outcome+`"`)) || h.Auth != "Bearer "+f.comp.Token {
		return fmt.Errorf("POST /v1/runs/%s/end: found=%v body %s, want the computer token and %q\ndaemon output:\n%s", f.run.RunID, ok, h.In, outcome, f.daemon.text())
	}
	return nil
}

// answersPermission: the agent asks permission; the daemon answers with this option, logs the request without the tool call's
// title, and the run still ends completed.
func (f *f09) answersPermission(record, option string) error {
	r, err := f.wait(record, "an answer to its permission request", func(r permRec) bool { return len(r.Permission) > 0 && string(r.Permission) != "null" })
	if err != nil {
		return err
	}
	var got struct {
		Outcome struct{ Outcome, OptionID string }
	}
	_ = json.Unmarshal(r.Permission, &got)
	if got.Outcome.Outcome != "selected" || got.Outcome.OptionID != option {
		return fmt.Errorf("the agent was answered %s, want the %s option", r.Permission, option)
	}
	if err := f.endedAs("completed"); err != nil {
		return err
	}
	if log := f.daemon.text(); !strings.Contains(log, "permission request") || !strings.Contains(log, option) || strings.Contains(log, "a tool call title") {
		return fmt.Errorf("the daemon's log must name the request and its answer and never the tool call's title:\n%s", log)
	}
	return nil
}

const askKinds = "permission=allow_always,allow_once,reject_always,reject_once"

// TestF09PermissionDeniedByDefault: an agent that asks permission is answered with the request's reject option.
func TestF09PermissionDeniedByDefault(t *testing.T) {
	needFake(t)
	par(t)
	track(t)
	f := newF09(t)
	record := filepath.Join(t.TempDir(), "agent.json")
	f.harness(record, askKinds)
	p := &stepper{out: os.Stdout, events: func() []Frame { return f.obs[0].snapshot(0) }}
	defer p.count(t)
	p.step("a woken agent asks permission", func() error { return f.wakes(false) })
	p.step("the daemon answers with the reject option, logs it without the title, and the run still ends completed", func() error { return f.answersPermission(record, "reject_once") })
	if p.ok != p.run {
		t.Fatalf("%d of %d steps passed", p.ok, p.run)
	}
}

// TestF09PermissionAllowedByTheAllowSetting: an agent added with --allow-permissions is answered allow_once, never allow_always.
func TestF09PermissionAllowedByTheAllowSetting(t *testing.T) {
	needFake(t)
	par(t)
	track(t)
	f := newF09(t)
	record := filepath.Join(t.TempDir(), "agent.json")
	f.harness(record, askKinds)
	p := &stepper{out: os.Stdout, events: func() []Frame { return f.obs[0].snapshot(0) }}
	defer p.count(t)
	p.step("an agent added with --allow-permissions is woken and asks permission", func() error { return f.wakes(true) })
	p.step("the daemon answers with allow_once", func() error { return f.answersPermission(record, "allow_once") })
	if p.ok != p.run {
		t.Fatalf("%d of %d steps passed", p.ok, p.run)
	}
}

// stopsTheRun: a person stops the run mid-turn; the agent gets session/cancel, leaves on its own, and the run ends cancelled.
func (f *f09) stopsTheRun(record string) error {
	if _, err := f.wait(record, "its prompt", func(r permRec) bool { return set(r.Prompt) }); err != nil {
		return err
	}
	if r := f.c.do("POST", "/v1/pods/"+f.pod+"/runs/"+f.run.RunID+"/stop", f.owner.tok, map[string]any{"reason": "stop now"}, nil); !r.ok() {
		return fmt.Errorf("POST run stop: HTTP %d", r.Status)
	}
	r, err := f.wait(record, "session/cancel", func(r permRec) bool { return r.Cancelled })
	if err != nil {
		return err
	}
	if r.Signalled {
		return fmt.Errorf("the agent was signalled; it ends its turn on session/cancel and must be left alone")
	}
	for end := time.Now().Add(5 * time.Second); syscall.Kill(-r.PID, 0) == nil; time.Sleep(25 * time.Millisecond) {
		if time.Now().After(end) {
			_ = syscall.Kill(-r.PID, syscall.SIGKILL)
			return fmt.Errorf("the agent's process group %d still runs 5 s after the stop", r.PID)
		}
	}
	return f.endedAs("cancelled")
}

// TestF09StopCancelsTheSession: run.stop reaches the daemon, which cancels the harness session before it signals anything.
func TestF09StopCancelsTheSession(t *testing.T) {
	needFake(t)
	par(t)
	track(t)
	f := newF09(t)
	record := filepath.Join(t.TempDir(), "agent.json")
	f.harness(record, "cancel")
	p := &stepper{out: os.Stdout, events: func() []Frame { return f.obs[0].snapshot(0) }}
	defer p.count(t)
	p.step("an agent mid-turn is woken", func() error { return f.wakes(false) })
	p.step("a person stops the run: the agent gets session/cancel, leaves unsignalled, and the pod sees the run end cancelled", func() error { return f.stopsTheRun(record) })
	if p.ok != p.run {
		t.Fatalf("%d of %d steps passed", p.ok, p.run)
	}
}
