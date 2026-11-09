package redact

import (
	"bytes"
	"strings"
	"testing"
)

func TestStringShortensEveryTokenPrefix(t *testing.T) {
	for _, prefix := range []string{"jmu_", "jmc_", "jmr_"} {
		got := String("before " + prefix + "AbC123xyz_-9Q after")
		if want := "before " + prefix + "… after"; got != want {
			t.Errorf("%s token: got %q, want %q", prefix, got, want)
		}
	}
	if got := String("jmx_AbC123xyz and jm_whoami"); got != "jmx_AbC123xyz and jm_whoami" {
		t.Errorf("text that is not a token changed: %q", got)
	}
}

// A Commonly device token (cm_) the importer holds is shortened too, if one ever reaches a line; "cm_" ending a word is left alone.
func TestStringShortensACommonlyToken(t *testing.T) {
	if got, want := String("token cm_0123456789abcdef end"), "token cm_… end"; got != want {
		t.Errorf("got %q, want %q", got, want)
	}
	if got := String("the vcm_ prefix and pcm_x"); got != "the vcm_ prefix and pcm_x" {
		t.Errorf("a word that only ends in cm_ changed: %q", got)
	}
}

func TestStringStripsControlCharactersButKeepsNewlineAndTab(t *testing.T) {
	cases := map[string]struct{ in, want string }{
		"ansi escape": {"a\x1b[2Jb\x1b]0;title\x07c", "a[2Jb]0;titlec"},
		"bare CR":     {"line one\rrewritten", "line onerewritten"},
		"NUL":         {"a\x00b", "ab"},
		"DEL":         {"a\x7fb", "ab"},
		"C1 control":  {"a\u009bb", "ab"},
		"kept":        {"a\tb\nc", "a\tb\nc"},
	}
	for name, c := range cases {
		if got := String(c.in); got != c.want {
			t.Errorf("%s: got %q, want %q", name, got, c.want)
		}
	}
}

func TestStringRedactsATokenSplitByControlCharacters(t *testing.T) {
	// The control characters go first, so a NUL planted inside a token cannot hide the rest of it.
	if got := String("jmc_ab\x00cd\ref\x1bgh"); got != "jmc_…" {
		t.Errorf("got %q, want the whole token shortened", got)
	}
}

func TestWriterRedactsAndReportsTheCallersLength(t *testing.T) {
	var buf bytes.Buffer
	w := Writer(&buf)
	in := "jmu_SECRETSECRET\x1b[31m\r\n"
	n, err := w.Write([]byte(in))
	if err != nil || n != len(in) {
		t.Fatalf("Write returned %d, %v; want %d, nil", n, err, len(in))
	}
	if got := buf.String(); got != "jmu_…[31m\n" || strings.Contains(got, "SECRET") {
		t.Errorf("wrote %q", got)
	}
}
