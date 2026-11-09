package importer

import (
	"fmt"
	"strings"
	"time"
	"unicode/utf8"

	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

const (
	summaryMax = 120
	bodyMax    = 16384
	markerKey  = "commonly-msg:"
	podNameMax = 80
	timeLayout = "2006-01-02T15:04:05.000Z"
)

// sourceMarker is the last line of an imported body. It is the only trace of the source id on the Jellymoon side, and what a
// resumed run looks for when a post may have landed without its answer reaching the importer.
func sourceMarker(id string) string { return markerKey + id }

// markerOf returns the marker on a body's last line, or "".
func markerOf(body string) string {
	body = strings.TrimRight(body, "\n ")
	last := body[strings.LastIndex(body, "\n")+1:]
	if strings.HasPrefix(last, markerKey) {
		return last
	}
	return ""
}

// canonTime puts a timestamp in one shape (UTC, milliseconds) so the cursor compares as text. Anything that does not parse is kept.
func canonTime(at string) string {
	if t, err := time.Parse(time.RFC3339Nano, at); err == nil {
		return t.UTC().Format(timeLayout)
	}
	return at
}

// compose is the post for one message. Jellymoon stamps its own author and time (docs/protocol.md), so the original author and
// date ride in a body prefix. Attachments are never fetched: each becomes a placeholder with the file's name only.
func compose(n Norm, author string) protocol.PostInput {
	var b strings.Builder
	fmt.Fprintf(&b, "[imported · %s · %s]\n\n", author, n.At)
	b.WriteString(clip(n.Text, bodyMax-1200))
	for _, f := range n.Files {
		fmt.Fprintf(&b, "\n[attachment: %s]", oneLine(f))
	}
	b.WriteString("\n\n" + sourceMarker(n.ID))
	body := b.String()
	return protocol.PostInput{Summary: summaryOf(n), Kind: protocol.MessageKindFyi, Status: protocol.MessageStatusDone, Body: &body}
}

func summaryOf(n Norm) string {
	for _, line := range strings.Split(n.Text, "\n") {
		if s := oneLine(line); s != "" {
			return clip(s, summaryMax)
		}
	}
	if len(n.Files) > 0 {
		return "(attachment) " + clip(oneLine(n.Files[0]), 80)
	}
	return "(empty message)"
}

func oneLine(s string) string { return strings.Join(strings.Fields(s), " ") }

// clip cuts s to at most max characters, marking the cut.
func clip(s string, max int) string {
	if utf8.RuneCountInString(s) <= max {
		return s
	}
	r := []rune(s)
	return string(r[:max-1]) + "…"
}

func podName(name string) string {
	if s := clip(oneLine(name), podNameMax); s != "" {
		return s
	}
	return "Imported pod"
}

// participant names the nth person "participant A", "participant B", … "participant AA".
func participant(n int) string {
	s := ""
	for n++; n > 0; n = (n - 1) / 26 {
		s = string(rune('A'+(n-1)%26)) + s
	}
	return "participant " + s
}
