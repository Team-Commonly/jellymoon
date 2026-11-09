package e2e

import (
	"fmt"
	"strings"
	"testing"
)

// TestLogBufTail: the tail is the last n lines, the lines that name a failure first, no stack dump, and no line longer
// than lineCap. A Worker that failed to start printed its cause, then one stack line over 1 KB, and the old byte tail
// showed only that stack line.
func TestLogBufTail(t *testing.T) {
	var l logBuf
	fmt.Fprintln(&l, "booting")
	fmt.Fprintln(&l, "*** Fatal uncaught kj::Exception: listen: address in use")
	for i := range 30 {
		fmt.Fprintf(&l, "line %d\n", i)
	}
	fmt.Fprintln(&l, "✘ [ERROR] The Workers runtime failed to start")
	fmt.Fprintln(&l, "  stack: "+strings.Repeat("x", 2000))
	fmt.Fprint(&l, strings.Repeat("y", 3000))
	got := strings.Split(l.tail(5), "\n")
	want := []string{"*** Fatal uncaught kj::Exception: listen: address in use", "✘ [ERROR] The Workers runtime failed to start", "line 28", "line 29"}
	if len(got) != 5 || strings.Join(got[:4], "|") != strings.Join(want, "|") {
		t.Fatalf("tail(5) = %q, want %q and then the capped last line", got, want)
	}
	if last := got[4]; len(last) > lineCap+3 || !strings.HasPrefix(last, "yyy") {
		t.Fatalf("the last line is %d bytes, want at most %d", len(last), lineCap+3)
	}
	var quiet logBuf
	fmt.Fprint(&quiet, "a\nb\nc\n")
	if got := quiet.tail(2); got != "b\nc" {
		t.Fatalf("tail(2) of a log with no failure = %q, want the last two lines", got)
	}
}

// TestStartRetry: a taken port and a Workers runtime that failed to start are retried, each with its own line; any
// other failure is not. The cause is the first line that names a failure.
func TestStartRetry(t *testing.T) {
	crash := "✘ [ERROR] The Workers runtime failed to start\n  stack: at x\n"
	for _, c := range []struct {
		out, line string
		retry     bool
	}{
		{"ERROR Address already in use\n", "PORT_RETRY test=T port=7 try=2", true},
		{"listen EADDRINUSE: 127.0.0.1:7\n", "PORT_RETRY test=T port=7 try=2", true},
		{crash, "START_RETRY test=T port=7 try=2 cause=✘ [ERROR] The Workers runtime failed to start", true},
		{"*** \x1b[31mFatal\x1b[0m uncaught kj::Exception: boom\n" + crash, "START_RETRY test=T port=7 try=2 cause=*** Fatal uncaught kj::Exception: boom", true},
		{"ERROR could not read the config\n", "", false},
	} {
		line, retry := startRetry("T", 7, 2, []byte(c.out))
		if line != c.line || retry != c.retry {
			t.Errorf("startRetry(%q) = %q %v, want %q %v", c.out, line, retry, c.line, c.retry)
		}
	}
}
