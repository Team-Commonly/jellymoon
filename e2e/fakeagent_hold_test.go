package e2e

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// The fake agent in hold mode is the "will not leave on its own" agent that TestF09 relies on. Once it has its prompt and
// nothing else runs it must stay up, not end with Go's "all goroutines are asleep" error, which is a way out.
func TestFakeAgentHoldStaysUp(t *testing.T) {
	par(t)
	root, _ := filepath.Abs("..")
	dir := t.TempDir()
	bin, record := filepath.Join(dir, "fakeagent"), filepath.Join(dir, "rec.json")
	build := exec.Command("go", "build", "-o", bin, "./e2e/testdata/fakeagent")
	build.Dir = root
	if out, err := build.CombinedOutput(); err != nil {
		t.Fatalf("go build ./e2e/testdata/fakeagent: %v\n%s", err, out)
	}
	cmd := exec.Command(bin, record, "hold")
	in, err := cmd.StdinPipe()
	if err != nil {
		t.Fatal(err)
	}
	if err := cmd.Start(); err != nil {
		t.Fatal(err)
	}
	exited := make(chan error, 1)
	go func() { exited <- cmd.Wait() }()
	t.Cleanup(func() { _ = cmd.Process.Kill() })
	_, _ = in.Write([]byte(`{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}` + "\n" +
		`{"jsonrpc":"2.0","id":2,"method":"session/new","params":{}}` + "\n" +
		`{"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{}}` + "\n"))
	// Wait on the condition, not on a clock: the record names the prompt once the agent has it. Under load a fixed deadline
	// can beat a slow process start, so the only ways out are the record appearing or the agent exiting (a real failure);
	// go test's own timeout bounds a hang.
	for recorded := false; !recorded; {
		select {
		case err := <-exited:
			t.Fatalf("the hold agent exited before it recorded its prompt (%v)", err)
		case <-time.After(10 * time.Millisecond):
			b, _ := os.ReadFile(record)
			recorded = strings.Contains(string(b), `"Prompt"`)
		}
	}
	select {
	case err := <-exited:
		t.Fatalf("the hold agent exited after its prompt (%v)", err)
	case <-time.After(500 * time.Millisecond):
	}
}
