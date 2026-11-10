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

// The shape takes an ellipsis String already left, so text that passes through twice (the reader's, then the CLI's writer) is
// the same length as text that passed once.
var tokenShape = regexp.MustCompile(`(jm[ucr]_|\bcm_)[A-Za-z0-9_-]*…?`)

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

// Replaced is what Credentials leaves where a credential was.
const Replaced = "[redacted]"

// credentialShape is the vendor key shapes worth naming: model provider and GitHub keys, Slack tokens, AWS key ids, JWTs and PEM
// private keys.
var credentialShape = regexp.MustCompile(`\bsk-[A-Za-z0-9_-]{16,}|\bgh[pousr]_[A-Za-z0-9]{20,}|\bgithub_pat_[A-Za-z0-9_]{20,}|\bxox[a-z]-[A-Za-z0-9-]{10,}|\bAKIA[0-9A-Z]{16}\b` +
	`|\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]*|(?s)-----BEGIN [A-Z ]*PRIVATE KEY-----.*?(-----END [A-Z ]*PRIVATE KEY-----|$)`)

// longRun is 40 or more characters of the base64 alphabets with no break: long enough to be a key of unknown make.
var longRun = regexp.MustCompile(`[A-Za-z0-9+/=_-]{40,}`)

// Credentials is String for text a person or an agent wrote, which can hold any vendor's key: it also replaces every known
// key shape, and any long unbroken run that mixes digits with both letter cases. A git hash (one case) and a long path of
// words are left alone.
func Credentials(s string) string {
	s = credentialShape.ReplaceAllString(String(s), Replaced)
	return longRun.ReplaceAllStringFunc(s, func(run string) string {
		if strings.ContainsAny(run, "0123456789") && strings.ToLower(run) != run && strings.ToUpper(run) != run {
			return Replaced
		}
		return run
	})
}
