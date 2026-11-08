package e2e

import (
	"encoding/json"
	"fmt"
	"net/url"
)

// forgedWho is what the Worker's own header looks like when someone else writes it: the human owner, as a user token would be stamped.
func (f *f11) forgedWho() string {
	raw, _ := json.Marshal(map[string]any{"kind": "user", "actor": f.owner.actor})
	return url.PathEscape(string(raw))
}

// forgedIdentity: the one header the pod trusts is set by the Worker alone. A caller who sends it too is still posted as the token's own caller.
func (f *f11) forgedIdentity() error {
	cases := []struct{ name, tok, author, run string }{
		{"a run token", f.run.RunToken, f.agentID, f.run.RunID},
		{"a member token", f.ben.tok, f.ben.actor.ID, ""},
	}
	for _, c := range cases {
		summary := "Forged identity from " + c.name
		st, raw := f.httpAs(c.tok, map[string]any{"summary": summary, "kind": "fyi", "status": "done"}, map[string]string{"X-Jm-Who": f.forgedWho()})
		if st != 200 {
			return fmt.Errorf("post with %s and a forged x-jm-who: HTTP %d %s, want 200", c.name, st, raw)
		}
		m, err := f.storedWith(summary)
		if err != nil {
			return err
		}
		if m.Author.ID != c.author || m.RunID != c.run || (c.run != "") != (m.Receipt != nil) {
			return fmt.Errorf("post with %s and x-jm-who naming the owner %s: stored author %s, run %q, receipt %v; want author %s, run %q and a receipt only for a run", c.name, f.owner.actor.ID, m.Author.ID, m.RunID, m.Receipt != nil, c.author, c.run)
		}
	}
	return nil
}
