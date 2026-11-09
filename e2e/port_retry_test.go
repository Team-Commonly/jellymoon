package e2e

import (
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"
	"testing"
)

// standIn is the server command TestPortRetry's child starts. Each start records its port, then does what the
// plan's next line says: "inuse" prints the bind error and exits 1, "other" exits 1 with a different error,
// "ok" serves 200 on every path. Past the plan's end it repeats the last line.
const standIn = `n=$(wc -l < "$JM_FAKE_DIR/starts" 2>/dev/null || echo 0); echo "$PORT" >> "$JM_FAKE_DIR/starts"
mode=$(sed -n "$((n+1))p" "$JM_FAKE_DIR/plan"); [ -n "$mode" ] || mode=$(tail -1 "$JM_FAKE_DIR/plan")
case "$mode" in
inuse) echo "ERROR Address already in use. Please check that you are not already running a server on this address"; exit 1 ;;
other) echo "ERROR could not read the config"; exit 1 ;;
crash) echo "*** Fatal uncaught kj::Exception: boom"; echo "[ERROR] The Workers runtime failed to start"; exit 1 ;;
ok) exec "$JM_FAKE_BUN" -e 'Bun.serve({ hostname: "127.0.0.1", port: Number(process.env.PORT), fetch: () => new Response("ok") }); setInterval(() => {}, 1e6)' ;;
esac`

// TestPortRetry drives startServer against a stand-in server in a child test process, so a failed start
// is observed the way the suite sees it. It needs no real server.
func TestPortRetry(t *testing.T) {
	bun, err := exec.LookPath("bun")
	if err != nil {
		t.Skip("bun is not on PATH; the stand-in server needs it")
	}
	cases := []struct {
		name, plan string
		pass       bool
		starts     int
		retries    int
	}{
		{"taken twice, then it starts", "inuse\ninuse\nok\n", true, 3, 2},
		{"taken every time: three tries, never a fourth", "inuse\n", false, 3, 2},
		{"another failure is not retried", "other\n", false, 1, 0},
		{"another failure after a taken port is not retried", "inuse\nother\n", false, 2, 1},
		{"the Workers runtime failed to start, then it starts", "crash\ninuse\nok\n", true, 3, 2},
	}
	retryLine := regexp.MustCompile(`(?m)^(?:PORT_RETRY|START_RETRY) test=TestPortRetryChild port=(\d+) try=(\d+)(?: cause=\*\*\* Fatal uncaught kj::Exception: boom)?$`)
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			dir := t.TempDir()
			if err := os.WriteFile(filepath.Join(dir, "plan"), []byte(c.plan), 0o600); err != nil {
				t.Fatal(err)
			}
			script := filepath.Join(dir, "server.sh")
			if err := os.WriteFile(script, []byte(standIn), 0o700); err != nil {
				t.Fatal(err)
			}
			child := exec.Command(os.Args[0], "-test.run=^TestPortRetryChild$", "-test.count=1", "-test.v")
			child.Env = append(os.Environ(), "JM_PORT_RETRY_CHILD=1", "JM_FAKE_DIR="+dir, "JM_FAKE_BUN="+bun, "JM_E2E_SERVER=sh "+script)
			out, err := child.CombinedOutput()
			if (err == nil) != c.pass {
				t.Fatalf("child passed=%v, want %v; output:\n%s", err == nil, c.pass, out)
			}
			raw, _ := os.ReadFile(filepath.Join(dir, "starts"))
			ports := strings.Fields(string(raw))
			if len(ports) != c.starts {
				t.Fatalf("%d starts, want %d; output:\n%s", len(ports), c.starts, out)
			}
			got := retryLine.FindAllStringSubmatch(string(out), -1)
			if len(got) != c.retries {
				t.Fatalf("%d PORT_RETRY lines, want %d; output:\n%s", len(got), c.retries, out)
			}
			for i, m := range got {
				if m[1] != ports[i] || m[2] != strconv.Itoa(i+2) {
					t.Fatalf("retry line %q, want port=%s try=%d", m[0], ports[i], i+2)
				}
				if ports[i+1] == ports[i] {
					t.Fatalf("a retry reused the taken port %s", ports[i])
				}
			}
			if !c.pass && !strings.Contains(string(out), "server exited before /healthz answered 200") {
				t.Fatalf("a failed start no longer fails with the old message; output:\n%s", out)
			}
		})
	}
}

// TestPortRetryChild is the child TestPortRetry runs; on its own it does nothing.
func TestPortRetryChild(t *testing.T) {
	if os.Getenv("JM_PORT_RETRY_CHILD") != "1" {
		t.Skip("run by TestPortRetry")
	}
	startServer(t)
}
