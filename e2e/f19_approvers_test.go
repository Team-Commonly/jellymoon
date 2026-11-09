package e2e

import (
	"encoding/json"
	"fmt"
	"slices"
	"strings"
	"time"
)

// The approver set of TestF19: besides the pod owners, a connection's owner-named members may approve its writes.

// writesSoon waits for the stub to have seen n writes: a released write's call runs after the ruling is announced.
func (f *f19) writesSoon(n int) error {
	var err error
	for end := time.Now().Add(5 * time.Second); time.Now().Before(end); time.Sleep(25 * time.Millisecond) {
		if err = f.writes(n); err == nil {
			return nil
		}
	}
	return err
}

// approvers sets the connection's approver set as `as` and returns the status and the stored set.
func (f *f19) approvers(as user, ids ...string) (int, []string, string) {
	if ids == nil {
		ids = []string{}
	}
	st, b := f.raw("POST", f.podPath("approvers"), as.tok, map[string]any{"connectionId": f.conn, "userIds": ids})
	var out struct {
		UserIDs []string `json:"userIds"`
	}
	_ = json.Unmarshal(b, &out)
	return st, out.UserIDs, string(b)
}

// approverSet: only an owner sets it, only members may be on it; a listed member approves, an unlisted member is refused, and
// an approver taken off the set is refused. It leaves the set empty, so step 5 sees an owner-only connection again.
func (f *f19) approverSet() error {
	w0 := len(f.linear.seen("create_issue"))
	if st, _, b := f.approvers(f.ben, f.ben.actor.ID); st != 403 || !strings.Contains(b, `"forbidden"`) {
		return fmt.Errorf("a member setting approvers: HTTP %d %s, want 403 forbidden", st, b)
	}
	if st, _, b := f.approvers(f.owner, "usr_00000000000000000000000000"); st != 404 {
		return fmt.Errorf("an approver who is not a pod member: HTTP %d %s, want 404", st, b)
	}
	if st, _, b := f.approvers(f.owner, f.ben.actor.ID, f.ben.actor.ID); st != 400 {
		return fmt.Errorf("an approver named twice: HTTP %d %s, want 400", st, b)
	}
	if st, ids, b := f.approvers(f.owner, f.ben.actor.ID); st != 200 || !slices.Equal(ids, []string{f.ben.actor.ID}) {
		return fmt.Errorf("the owner naming ben: HTTP %d %s, want ben's id alone", st, b)
	}
	listed, _, err := f.park(map[string]any{"title": "Approved by a listed member", "teamId": "TEAM-1"})
	if err != nil {
		return err
	}
	if err := f.rule(f.sCara, listed, "a", "forbidden"); err != nil {
		return fmt.Errorf("an unlisted member's approval: %w", err)
	}
	if err := f.writes(w0); err != nil {
		return fmt.Errorf("after an unlisted member's approval: %w", err)
	}
	if err := f.rule(f.sBen, listed, "a", "ruled"); err != nil {
		return fmt.Errorf("a listed member's approval: %w", err)
	}
	if err := f.writesSoon(w0 + 1); err != nil {
		return fmt.Errorf("after a listed member's approval: %w", err)
	}
	if r, b, err := f.receiptSoon(listed); err != nil || r.Approver == nil || r.Approver.ID != f.ben.actor.ID {
		return fmt.Errorf("the listed member's release: %s, want ben as its approver (%v)", b, err)
	}
	if st, ids, b := f.approvers(f.owner); st != 200 || len(ids) != 0 {
		return fmt.Errorf("the owner clearing the set: HTTP %d %s, want an empty set", st, b)
	}
	removed, _, err := f.park(map[string]any{"title": "Ruled on by a removed approver", "teamId": "TEAM-1"})
	if err != nil {
		return err
	}
	if err := f.rule(f.sBen, removed, "a", "forbidden"); err != nil {
		return fmt.Errorf("a removed approver's approval: %w", err)
	}
	if err := f.rule(f.sO, removed, "b", "ruled"); err != nil {
		return err
	}
	return f.writes(w0 + 1)
}

// receiptSoon reads the receipt of a released write once its upstream answer is recorded.
func (f *f19) receiptSoon(dec string) (connReceipt, string, error) {
	var (
		r   connReceipt
		b   string
		err error
	)
	for end := time.Now().Add(5 * time.Second); time.Now().Before(end); time.Sleep(25 * time.Millisecond) {
		if r, b, err = f.receiptOf(dec); err == nil && r.UpstreamStatus != 0 {
			return r, b, nil
		}
	}
	if err == nil {
		err = fmt.Errorf("no upstream status recorded within 5 s")
	}
	return r, b, err
}
