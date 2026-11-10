// Package reading turns a reader's day into page data with one reading call to the person's own harness, and checks what
// comes back (ADR 0011, section 2). The model is not trusted: a day can hold any text an agent ever printed. So the call can
// do nothing but return text (call.go), and here code keeps a wish or a clash only when its words are the person's own, writes
// the day, the ids and what was read itself, and cleans and cuts the few words that are the model's.
package reading

import (
	"encoding/json"
	"errors"
	"fmt"
	"slices"
	"strings"
	"time"

	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
	"github.com/Team-Commonly/jellymoon/daemon/internal/reader"
	"github.com/Team-Commonly/jellymoon/daemon/internal/redact"
)

// The limits of page data (schema/page.json).
const (
	wishesMax, clashesMax, starsMax = 8, 2, 4
	labelMax, starMax, quoteMax     = 40, 60, 200
)

// Options is one reading: the model's name, the folder the call's own empty folder is made in, and how long the call may take.
type Options struct {
	Model   string
	Dir     string
	Timeout time.Duration
}

// Page returns the day's page data. A day with no turns is an empty page and makes no call. A reading that is not page data
// is an error: there is no page to print.
func Page(day protocol.ReaderDay, o Options) (protocol.PageData, error) {
	page := protocol.PageData{Day: day.Day, Wishes: []protocol.Wish{}, Clashes: []protocol.Clash{}, Read: protocol.PageRead{Turns: int64(len(day.Turns)), Projects: []string{}}}
	for _, t := range day.Turns {
		if !slices.Contains(page.Read.Projects, t.Project) {
			page.Read.Projects = append(page.Read.Projects, t.Project)
		}
	}
	if !modelShape.MatchString(o.Model) {
		return page, errors.New("--model must be a model's name: letters, digits, dot, dash and underscore")
	}
	if len(day.Turns) == 0 {
		return page, nil
	}
	text, err := call(day, o)
	if err != nil {
		return page, err
	}
	var read struct {
		Wishes  *[]protocol.Wish  `json:"wishes"`
		Clashes *[]protocol.Clash `json:"clashes"`
	}
	if err := json.Unmarshal([]byte(unfence(text)), &read); err != nil {
		return page, fmt.Errorf("the reading is not page data: %w", err)
	}
	if read.Wishes == nil || read.Clashes == nil || slices.ContainsFunc(*read.Wishes, func(w protocol.Wish) bool { return !w.State.Valid() }) {
		return page, errors.New("the reading is not page data: it needs wishes, each in a known state, and clashes")
	}
	quote := func(s string) (string, bool) { return typed(day, s) }
	ids := map[string]string{} // the model's id of a kept wish, to the page's
	for _, w := range *read.Wishes {
		_, taken := ids[w.ID]
		said, ok := quote(w.Said)
		if w.Label = clean(w.Label, labelMax); taken || !ok || w.Label == "" || !slices.Contains(page.Read.Projects, w.Project) || len(page.Wishes) == wishesMax {
			continue
		}
		stars := []string{}
		for _, s := range w.Stars {
			if s = clean(s, starMax); s != "" && len(stars) < starsMax {
				stars = append(stars, s)
			}
		}
		ids[w.ID] = fmt.Sprintf("w%d", len(page.Wishes)+1)
		w.ID, w.Said, w.Stars = ids[w.ID], said, stars
		page.Wishes = append(page.Wishes, w)
	}
	for _, c := range *read.Clashes {
		a, b := ids[c.A], ids[c.B]
		qa, okA := quote(c.QuoteA)
		qb, okB := quote(c.QuoteB)
		if a != "" && b != "" && a != b && okA && okB && len(page.Clashes) < clashesMax {
			page.Clashes = append(page.Clashes, protocol.Clash{A: a, B: b, QuoteA: qa, QuoteB: qb, Settled: c.Settled})
		}
	}
	return page, nil
}

// unfence takes the model's text out of a Markdown code fence, when it put one around it.
func unfence(text string) string {
	text = strings.TrimSpace(text)
	if first, rest, ok := strings.Cut(text, "\n"); ok && strings.HasPrefix(first, "```") {
		return strings.TrimSuffix(strings.TrimSpace(rest), "```")
	}
	return text
}

// typed reports whether q is the person's own words: an exact substring of one thing they typed that day. A quote over its
// length is cut first, with no mark added, so what is left is still theirs.
func typed(day protocol.ReaderDay, q string) (string, bool) {
	if r := []rune(q); len(r) > quoteMax {
		q = string(r[:quoteMax])
	}
	q = strings.TrimSpace(q)
	return q, q != "" && slices.ContainsFunc(day.Turns, func(t protocol.ReaderTurn) bool { return strings.Contains(t.Said, q) })
}

// clean makes the model's own words safe to show: no control characters, one line, credential shapes replaced, cut to max.
func clean(s string, max int) string {
	return reader.Cut(strings.Join(strings.Fields(redact.Credentials(s)), " "), max)
}
