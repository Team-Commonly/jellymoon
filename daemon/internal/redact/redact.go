// Package redact is the one place everything the daemon or the jellymoon CLI prints passes through.
package redact

import "io"

// String is not redacting yet.
func String(s string) string { return s }

// Writer is not redacting yet.
func Writer(w io.Writer) io.Writer { return w }
