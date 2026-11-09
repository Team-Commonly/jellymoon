package e2e

import (
	"encoding/json"
	"fmt"
	"net/url"
	"reflect"
	"strings"
	"time"
)

// The write half of TestF19: a write parks as an ask, one approval releases it once, and its receipt links the ruling.

// card is the ask the pod posts for a parked write, as members receive it.
type card struct {
	ID, Kind, Summary, Body string
	Author                  Actor
	Options                 []struct{ ID, Label string }
	Decision                struct{ ID, State string }
}

const parkMs = 24 * 60 * 60 * 1000

// park asks for a write and returns the decision and the card members received for it.
func (f *f19) park(args map[string]any) (string, card, error) {
	mk := f.sO.mark()
	out, isErr, b := f.connector("create_issue", args)
	f.keep("the parked answer", b)
	dec, _ := out["decisionId"].(string)
	if isErr || out["state"] != "parked" || !strings.HasPrefix(dec, "dec_") {
		return "", card{}, fmt.Errorf("a write on a write-with-confirm grant: %s, want state parked with a dec_ decisionId", b)
	}
	var c card
	fr, ok := f.sO.waitAfter(mk, func(fr Frame) bool {
		fr.dec(&c)
		return fr.Type == "message.created" && c.Decision.ID == dec
	}, 5*time.Second)
	if !ok {
		return "", card{}, fmt.Errorf("no ask card with decision %s reached the owner", dec)
	}
	f.keep("the card "+dec, fr.Data)
	return dec, c, nil
}

// fenced reads the JSON the card's body shows between ```json and the closing fence.
func fenced(body string) (map[string]any, error) {
	_, rest, ok := strings.Cut(body, "```json\n")
	js, _, ok2 := strings.Cut(rest, "\n```")
	var v map[string]any
	if !ok || !ok2 || json.Unmarshal([]byte(js), &v) != nil {
		return nil, fmt.Errorf("the card body shows no ```json block of the arguments: %q", body)
	}
	return v, nil
}

// Step 4: a write parks; the card shows the exact arguments, approve and reject; the stub saw no write.
func (f *f19) parks() error {
	if err := f.grant("write-with-confirm"); err != nil {
		return err
	}
	f.args = map[string]any{"title": "Fix the login redirect", "teamId": "TEAM-1", "description": "Found while triaging; see the board."}
	dec, c, err := f.park(f.args)
	if err != nil {
		return err
	}
	f.parked = dec
	shown, err := fenced(c.Body)
	if err != nil {
		return err
	}
	if !reflect.DeepEqual(shown, f.args) || c.Kind != "ask" || c.Author.ID != f.a1.id || len(c.Options) != 2 || c.Options[0].ID != "a" || c.Options[1].ID != "b" {
		return fmt.Errorf("the card: kind %s by %s, options %+v, args %v; want agent-one's ask, a approve and b reject, args %v", c.Kind, c.Author.ID, c.Options, shown, f.args)
	}
	if n := len(f.linear.seen("create_issue")); n != 0 {
		return fmt.Errorf("the stub got %d writes while the call was parked", n)
	}
	if err := f.reclassified(); err != nil {
		return err
	}
	return f.noLeak()
}

// reclassified: a pinned write whose arguments claim it is a read (and whose upstream annotates it read-only) still parks.
func (f *f19) reclassified() error {
	dec, _, err := f.park(map[string]any{"title": "Claims to be a read", "teamId": "TEAM-1", "class": "read", "_class": "read", "readOnlyHint": true})
	if err != nil {
		return fmt.Errorf("a write claiming to be a read: %w", err)
	}
	if n := len(f.linear.seen("create_issue")); n != 0 {
		return fmt.Errorf("a write claiming to be a read reached the stub (%d writes)", n)
	}
	return f.rule(f.sO, dec, "b", "ruled")
}

// rule sends a ruling from s and waits for the decision to reach `state` (or for the error `code` on s).
func (f *f19) rule(s *sock, dec, option, want string) error {
	mk := s.mark()
	s.send("decision.rule", map[string]any{"decisionId": dec, "optionId": option})
	match := func(fr Frame) bool {
		var u ruledData
		fr.dec(&u)
		return fr.Type == "message.updated" && u.Decision.ID == dec && u.Decision.State == want
	}
	if want == "already_ruled" || want == "gone" || want == "forbidden" {
		match = errorWith(want)
	}
	if _, ok := s.waitAfter(mk, match, 5*time.Second); !ok {
		return fmt.Errorf("ruling %s on %s: no %s on %s within 5 s", option, dec, want, s.name)
	}
	return nil
}

func (f *f19) writes(n int) error {
	got := f.linear.seen("create_issue")
	if len(got) != n {
		return fmt.Errorf("the stub got %d writes, want %d: %+v", len(got), n, got)
	}
	return nil
}

// Step 5: a member who is not an owner cannot approve; the owner's approval releases one write with the exact arguments;
// a second ruling, a rejection and an expiry release none.
func (f *f19) releasesOnce() error {
	if err := f.rule(f.sBen, f.parked, "a", "forbidden"); err != nil {
		return fmt.Errorf("a plain member's approval: %w", err)
	}
	if err := f.writes(0); err != nil {
		return err
	}
	if err := f.rule(f.sO, f.parked, "a", "ruled"); err != nil {
		return err
	}
	if err := f.writesSoon(1); err != nil {
		return err
	}
	if w := f.linear.seen("create_issue")[0]; !reflect.DeepEqual(w.Args, f.args) || w.Bearer != "Bearer "+stubToken {
		return fmt.Errorf("the released write: args %v bearer ok %v, want exactly %v with the stub bearer", w.Args, w.Bearer == "Bearer "+stubToken, f.args)
	}
	if err := f.rule(f.sO, f.parked, "a", "already_ruled"); err != nil {
		return err
	}
	rejected, _, err := f.park(map[string]any{"title": "A second issue", "teamId": "TEAM-1"})
	if err != nil {
		return err
	}
	if err := f.rule(f.sO, rejected, "b", "ruled"); err != nil {
		return err
	}
	expired, _, err := f.park(map[string]any{"title": "A third issue", "teamId": "TEAM-1"})
	if err != nil {
		return err
	}
	mk := f.sO.mark()
	f.advance(parkMs + leaseMs)
	if _, ok := f.sO.waitAfter(mk, func(fr Frame) bool {
		var u ruledData
		fr.dec(&u)
		return fr.Type == "message.updated" && u.Decision.ID == expired && u.Decision.State == "expired"
	}, 5*time.Second); !ok {
		return fmt.Errorf("the third write's decision did not expire 24 h after it parked")
	}
	if err := f.rule(f.sO, expired, "a", "gone"); err != nil {
		return err
	}
	f.rejected, f.expired = rejected, expired
	return f.writes(1)
}

func (f *f19) receiptOf(dec string) (connReceipt, string, error) {
	st, b := f.raw("GET", f.podPath("connectors", "receipts")+"?decisionId="+url.QueryEscape(dec), f.owner.tok, nil)
	var out struct{ Receipts []connReceipt }
	if st != 200 || json.Unmarshal(b, &out) != nil || len(out.Receipts) != 1 {
		return connReceipt{}, string(b), fmt.Errorf("receipts for %s: HTTP %d %s, want exactly one", dec, st, b)
	}
	return out.Receipts[0], string(b), nil
}

// Step 6: the released write's receipt is the server's: it names the decision, the approver and the upstream's result.
func (f *f19) receipt() error {
	r, b, err := f.receiptSoon(f.parked)
	if err != nil {
		return err
	}
	issue, _ := r.Result["issue"].(map[string]any)
	if r.Outcome != "released" || r.DecisionID != f.parked || r.Approver == nil || r.Approver.ID != f.owner.actor.ID || issue["id"] != "ISS-2" || r.UpstreamStatus != 200 {
		return fmt.Errorf("the released write's receipt %s: want outcome released, the decision, the owner as approver, the stub's ISS-2 and status 200", b)
	}
	if r.ArgsHash != canonHash(f.args) || r.RunID != f.r.id || r.AgentID != f.a1.id || r.ConnectionID != f.conn || r.PodID != f.pod || !strings.HasPrefix(r.ID, "crc_") || !stamp.MatchString(r.CreatedAt) || !stamp.MatchString(r.SettledAt) {
		return fmt.Errorf("the receipt's stamps %s: want argsHash %s, run %s, agent %s, connection %s and RFC 3339 times", b, canonHash(f.args), f.r.id, f.a1.id, f.conn)
	}
	for dec, want := range map[string]string{f.rejected: "rejected", f.expired: "expired"} {
		other, ob, err := f.receiptOf(dec)
		if err != nil || other.Outcome != want || other.Approver != nil {
			return fmt.Errorf("the %s write's receipt %s: want outcome %s and no approver (%v)", want, ob, want, err)
		}
	}
	if st, _ := f.raw("POST", f.podPath("connectors", "receipts"), f.r.tok, map[string]any{"outcome": "released"}); st != 404 {
		return fmt.Errorf("a POST of a receipt: HTTP %d, want 404 (no route writes a receipt)", st)
	}
	if form := leakIn([]byte(b)); form != "" {
		return fmt.Errorf("the Linear token (%s) is in the receipt", form)
	}
	return nil
}
