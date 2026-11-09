// Package redact is the one place everything the daemon or the jellymoon CLI prints passes through. It shortens any
// jmu_, jmc_, jmr_ or Commonly cm_ token to its prefix and an ellipsis, and drops control characters other than newline and tab, so a
// string a server supplied can neither leak a token nor move the cursor or rewrite the terminal.
package redact

import (
	"io"
	"regexp"
	"strings"
	"unicode"
)

var tokenShape = regexp.MustCompile(`(jm[ucr]_|\bcm_)[A-Za-z0-9_-]*`)

// String strips control characters (all but newline and tab) and shortens every token to its prefix and an ellipsis. The
// control characters go first, so one planted inside a token cannot hide the rest of it.
func String(s string) string {
	s = strings.Map(func(r rune) rune {
		if unicode.IsControl(r) && r != '\n' && r != '\t' {
			return -1
		}
		return r
	}, s)
	return tokenShape.ReplaceAllString(s, "${1}…")
}

type writer struct{ w io.Writer }

// Writer returns a writer that passes each Write through String. Every writer here gets whole lines (fmt and log write one
// call per message), so a token is never split across two calls. It reports the caller's length, not the shortened one.
func Writer(w io.Writer) io.Writer { return writer{w} }

func (r writer) Write(p []byte) (int, error) {
	if _, err := r.w.Write([]byte(String(string(p)))); err != nil {
		return 0, err
	}
	return len(p), nil
}
