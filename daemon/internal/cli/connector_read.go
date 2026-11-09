package cli

import (
	"errors"
	"flag"
	"fmt"
	"io"
	"strings"

	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

// connectorList shows each connection with what its pinned operations let an agent call at each access level. The route gives a
// person the pin and no grants, so this is what a grant would allow, not who holds one. It never shows a credential: the
// server has none to give.
func connectorList(args []string, stdout, stderr io.Writer) error {
	_, pod, err := parse("list", args, func(*flag.FlagSet) {})
	if err != nil {
		return err
	}
	s, err := open(*pod)
	if err != nil {
		return err
	}
	views, err := s.a.Connectors(s.token, s.pod)
	if err != nil {
		return refusal(err)
	}
	if len(views) == 0 {
		fmt.Fprintln(stdout, "no connections")
	}
	for _, v := range views {
		var reads, writes []string
		for _, o := range v.Operations {
			if o.Class == protocol.OperationClassRead {
				reads = append(reads, o.Name)
			} else {
				writes = append(writes, o.Name)
			}
		}
		fmt.Fprintf(stdout, "%s  %s\n  read grant:               %s\n  write-with-confirm grant: also %s (each write waits for an approval)\n",
			v.ConnectionID, v.Vendor, orNone(reads), orNone(writes))
	}
	return nil
}

func orNone(names []string) string {
	if len(names) == 0 {
		return "none pinned"
	}
	return strings.Join(names, ", ")
}

// connectorReceipts prints the server's receipts, newest first, one per line.
func connectorReceipts(args []string, stdout, stderr io.Writer) error {
	var decision *string
	var limit *int
	_, pod, err := parse("receipts", args, func(fs *flag.FlagSet) {
		decision = fs.String("decision", "", "only the receipt of the write this decision holds")
		limit = fs.Int("limit", 0, "at most this many, 1 to 100 (default: the server's 20)")
	})
	if err != nil {
		return err
	}
	if *limit < 0 || *limit > 100 {
		return errors.New("--limit must be from 1 to 100")
	}
	var dec *protocol.DecisionID
	if *decision != "" {
		dec = decision
	}
	s, err := open(*pod)
	if err != nil {
		return err
	}
	rows, err := s.a.ConnectorReceipts(s.token, s.pod, dec, *limit)
	if err != nil {
		return refusal(err)
	}
	if len(rows) == 0 {
		fmt.Fprintln(stdout, "no receipts")
	}
	for _, r := range rows {
		fmt.Fprintln(stdout, receiptLine(r))
	}
	return nil
}

func receiptLine(r protocol.ConnectorReceipt) string {
	parts := []string{r.ID, string(r.Outcome), r.Operation, "agent=" + r.AgentID}
	if r.DecisionID != nil {
		parts = append(parts, "decision="+*r.DecisionID)
	}
	if r.Approver != nil {
		parts = append(parts, "approver="+r.Approver.ID)
	}
	if r.UpstreamStatus != nil {
		parts = append(parts, fmt.Sprintf("upstream=%d", *r.UpstreamStatus))
	}
	if r.Reason != nil {
		parts = append(parts, "reason="+strings.ReplaceAll(*r.Reason, "\n", " "))
	}
	return strings.Join(parts, "  ")
}
