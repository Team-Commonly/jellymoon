package e2e

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"

	"github.com/Team-Commonly/jellymoon/e2e/cmyfake"
)

const impSentinel = "cm_e2eSENTINELtoken0123456789"

// impRun is one importer process: its output, exit code and the files it left.
type impRun struct {
	out, errOut string
	code        int
}

type impEnv struct {
	t           *testing.T
	bin         string
	jm          *server
	jmURL       string
	jmTok       string
	home, work  string
	all         []string // everything any run printed or wrote, for the token scan
	instanceURL string
	rec         *recorder
	cmys        []*cmyfake.Server // every fixture instance the steps used, for the checks across all of them
}

func (e *impEnv) cmd(args ...string) *exec.Cmd {
	cmd := exec.Command(e.bin, append([]string{"import", "commonly", "--instance", e.instanceURL, "--pace-ms", "250", "--out", filepath.Join(e.work, "out")}, args...)...)
	cmd.Dir = e.work
	cmd.Env = []string{"PATH=" + os.Getenv("PATH"), "JELLYMOON_HOME=" + e.home, "JELLYMOON_SERVER=" + e.jmURL, "JELLYMOON_USER_TOKEN=" + e.jmTok, "COMMONLY_TOKEN=" + impSentinel}
	return cmd
}

func (e *impEnv) finish(cmd *exec.Cmd, so, se *bytes.Buffer, err error) impRun {
	code := 0
	if ee, ok := err.(*exec.ExitError); ok {
		code = ee.ExitCode()
		if code < 0 {
			code = -1
		}
	} else if err != nil {
		e.t.Fatalf("running the importer: %v", err)
	}
	e.all = append(e.all, so.String(), se.String())
	return impRun{so.String(), se.String(), code}
}

func (e *impEnv) run(args ...string) impRun {
	cmd := e.cmd(args...)
	var so, se bytes.Buffer
	cmd.Stdout, cmd.Stderr = &so, &se
	return e.finish(cmd, &so, &se, cmd.Run())
}

// fresh gives each step its own fixture instance, config dir and work dir, on the shared Jellymoon server.
func (e *impEnv) fresh(c *cmyfake.Server) {
	e.cmys = append(e.cmys, c)
	e.instanceURL = c.Serve(e.t).URL
	e.home, e.work = filepath.Join(e.t.TempDir(), "jm"), e.t.TempDir()
}

func (e *impEnv) scanFiles() {
	for _, dir := range []string{e.home, e.work} {
		_ = filepath.Walk(dir, func(p string, fi os.FileInfo, err error) error {
			if err == nil && fi.Mode().IsRegular() && filepath.Base(p) != "user-token" {
				b, _ := os.ReadFile(p)
				e.all = append(e.all, string(b))
			}
			return nil
		})
	}
}

// jmPods reads the Jellymoon pod ids the importer's state file maps the Commonly pods to.
func (e *impEnv) jmPods() map[string]string {
	raw, err := os.ReadFile(filepath.Join(e.home, "import-commonly.json"))
	if err != nil {
		return nil
	}
	var st struct {
		Pods map[string]struct{ JM string } `json:"pods"`
	}
	_ = json.Unmarshal(raw, &st)
	out := map[string]string{}
	for k, v := range st.Pods {
		out[k] = v.JM
	}
	return out
}

// feed returns the bodies of every message in a Jellymoon pod.
func (e *impEnv) feed(pod string) []string {
	var out struct {
		Messages []struct {
			Body string `json:"body"`
			Seq  int    `json:"seq"`
		} `json:"messages"`
	}
	c := newClient(e.t, e.jmURL)
	var bodies []string
	for since := 0; ; {
		out.Messages = nil
		path := fmt.Sprintf("/v1/pods/%s/feed?withBody=true&limit=50&sinceSeq=%d", pod, since)
		if r := c.do("GET", path, e.jmTok, nil, &out); !r.ok() {
			e.t.Fatalf("GET %s: HTTP %d", path, r.Status)
		}
		if len(out.Messages) == 0 {
			return bodies
		}
		for _, m := range out.Messages {
			bodies = append(bodies, m.Body)
			since = max(since, m.Seq)
		}
	}
}

func (e *impEnv) markers() map[string]int {
	got := map[string]int{}
	for _, jm := range e.jmPods() {
		for _, b := range e.feed(jm) {
			lines := strings.Split(strings.TrimSpace(b), "\n")
			got[lines[len(lines)-1]]++
		}
	}
	return got
}

// writes are the POSTs the recorder saw: the importer's only way to change Jellymoon.
func (r *recorder) writes() []hit {
	var out []hit
	for _, h := range r.hitsAll() {
		if h.Method == "POST" {
			out = append(out, h)
		}
	}
	return out
}

func (r *recorder) hitsAll() []hit {
	r.mu.Lock()
	defer r.mu.Unlock()
	return append([]hit(nil), r.hits...)
}

// dump is every request the recorder saw, headers and bodies, for the token scan.
func (r *recorder) dump() string {
	var b strings.Builder
	for _, h := range r.hitsAll() {
		fmt.Fprintf(&b, "%s %s %s\n%s\n", h.Method, h.Path, h.Auth, h.In)
	}
	return b.String()
}

// writes counts the POSTs Jellymoon saw: the importer's only way to change it.
func (e *impEnv) writes() int { return len(e.rec.writes()) }
