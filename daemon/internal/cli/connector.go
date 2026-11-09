package cli

import (
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"strings"

	"github.com/Team-Commonly/jellymoon/daemon/internal/api"
	"github.com/Team-Commonly/jellymoon/daemon/internal/config"
	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

// connectorTokenEnv carries the upstream credential for `connector connect`. It never comes from argv.
const connectorTokenEnv = "JELLYMOON_CONNECTOR_TOKEN"

// session is what every connector command needs: the server, the person's jmu_ token and the pod.
type session struct {
	a     *api.Client
	token string
	pod   string
}

// connectorCmd is `jellymoon connector connect|grant|approvers|list|receipts` (ADR 0009): a pod owner's connect, grant and approvers,
// and a member's list and receipts, over the server's existing routes. Every command checks its own arguments before it
// reads a credential or makes a request, and never echoes an argument it refuses.
func connectorCmd(args []string, stdout, stderr io.Writer) error {
	if len(args) == 0 {
		return errors.New("connector needs connect, grant, approvers, list or receipts; run jellymoon with no arguments for usage")
	}
	run := map[string]func([]string, io.Writer, io.Writer) error{
		"connect": connectorConnect, "grant": connectorGrant, "approvers": connectorApprovers, "list": connectorList, "receipts": connectorReceipts,
	}[args[0]]
	if run == nil {
		return errors.New("connector needs connect, grant, approvers, list or receipts")
	}
	return run(args[1:], stdout, stderr)
}

// parse reads the flags of one subcommand. A stray argument is refused without being printed: it may be a pasted credential.
// The flag package's own output and errors quote the argument (a flag name, a value it rejects), so neither is shown: a parse
// error becomes a fixed message and the subcommand's usage.
func parse(name string, args []string, set func(*flag.FlagSet)) (*flag.FlagSet, *string, error) {
	fs := flag.NewFlagSet("connector "+name, flag.ContinueOnError)
	fs.SetOutput(io.Discard)
	fs.Usage = func() {}
	pod := fs.String("pod", "", "the pod")
	set(fs)
	if err := fs.Parse(args); err != nil {
		return nil, nil, fmt.Errorf("unknown flag or bad flag value (not repeated here; it may be a credential)\nusage:\n%s", usageOf("jellymoon connector "+name+" "))
	}
	if fs.NArg() > 0 {
		return nil, nil, errors.New("unexpected extra argument (a credential is never an argument)")
	}
	if !podIDShape.MatchString(*pod) {
		return nil, nil, errors.New("--pod must be a pod id (pod_ and 26 characters)")
	}
	return fs, pod, nil
}

// open connects to the server as the person. It runs after the arguments and any credential have been checked.
func open(pod string) (session, error) {
	server := os.Getenv(serverEnv)
	if server == "" {
		return session{}, fmt.Errorf("set %s to your Jellymoon server's URL", serverEnv)
	}
	a, err := api.New(server)
	if err != nil {
		return session{}, err
	}
	store, err := config.Open()
	if err != nil {
		return session{}, err
	}
	tok, err := store.UserToken(userTokenEnv)
	if err != nil {
		return session{}, fmt.Errorf("%w (run jellymoon login first)", err)
	}
	return session{a: a, token: tok, pod: pod}, nil
}

// refusal words a server error: its code and message as sent, and for unauthorized the way back in.
func refusal(err error) error {
	var e *api.Error
	if errors.As(err, &e) && e.Body.Code == protocol.ErrorCodeUnauthorized {
		return fmt.Errorf("%s: %s (run jellymoon login again)", e.Body.Code, e.Body.Message)
	}
	if errors.As(err, &e) {
		return fmt.Errorf("%s: %s", e.Body.Code, e.Body.Message)
	}
	return err
}

// usageOf returns the usage lines that start with prefix, so a refusal shows one command's usage, not the whole list.
func usageOf(prefix string) string {
	var b strings.Builder
	for _, l := range strings.Split(usage, "\n") {
		if strings.HasPrefix(strings.TrimSpace(l), prefix) {
			b.WriteString(l + "\n")
		}
	}
	return strings.TrimSuffix(b.String(), "\n")
}

func connectorConnect(args []string, stdout, stderr io.Writer) error {
	if len(args) == 0 || strings.HasPrefix(args[0], "-") {
		return errors.New("connect needs a vendor first, such as linear")
	}
	vendor := protocol.ConnectorVendor(args[0])
	if !vendor.Valid() {
		return errors.New("unknown vendor; the supported one is linear")
	}
	var upstream *string
	fs, pod, err := parse("connect", args[1:], func(fs *flag.FlagSet) {
		upstream = fs.String("url", "", "the upstream's endpoint, https")
	})
	if err != nil {
		return err
	}
	_ = fs
	if *upstream == "" {
		return errors.New("--url is required, such as the vendor's remote MCP URL")
	}
	tok, err := readSecret(connectorTokenEnv, "Upstream token")
	if err != nil {
		return err
	}
	s, err := open(*pod)
	if err != nil {
		return err
	}
	c, err := s.a.Connect(s.token, s.pod, protocol.ConnectRequest{Vendor: vendor, URL: *upstream, Token: tok})
	if err != nil {
		// A server's text is scrubbed of the credential already; this is the second lock on that door.
		return errors.New(strings.ReplaceAll(refusal(err).Error(), tok, "[credential]"))
	}
	fmt.Fprintf(stdout, "connected %s as %s\n", c.Vendor, c.ID)
	for _, o := range c.Operations {
		fmt.Fprintf(stdout, "  %-14s %s\n", o.Name, o.Class)
	}
	return nil
}

func connectorGrant(args []string, stdout, stderr io.Writer) error {
	var conn, agent, access *string
	_, pod, err := parse("grant", args, func(fs *flag.FlagSet) {
		conn = fs.String("connection", "", "the connection id")
		agent = fs.String("agent", "", "the agent id")
		access = fs.String("access", "", "read or write-with-confirm")
	})
	if err != nil {
		return err
	}
	switch protocol.ConnectorAccess(*access) {
	case protocol.ConnectorAccessRead, protocol.ConnectorAccessWriteWithConfirm:
	default:
		if *access == "write" {
			return errors.New("there is no plain write; use --access write-with-confirm (each write waits for an approval)")
		}
		return errors.New("--access must be read or write-with-confirm")
	}
	if *conn == "" || *agent == "" {
		return errors.New("--connection and --agent are required")
	}
	s, err := open(*pod)
	if err != nil {
		return err
	}
	g, err := s.a.Grant(s.token, s.pod, protocol.GrantRequest{ConnectionID: *conn, AgentID: *agent, Access: protocol.ConnectorAccess(*access)})
	if err != nil {
		return refusal(err)
	}
	fmt.Fprintf(stdout, "granted %s %s on %s\n", g.AgentID, g.Access, g.ConnectionID)
	return nil
}

func connectorApprovers(args []string, stdout, stderr io.Writer) error {
	var conn, users *string
	fs, pod, err := parse("approvers", args, func(fs *flag.FlagSet) {
		conn = fs.String("connection", "", "the connection id")
		users = fs.String("users", "", "the pod members who may approve its writes, comma separated; \"\" clears the set")
	})
	if err != nil {
		return err
	}
	given := false
	fs.Visit(func(f *flag.Flag) { given = given || f.Name == "users" })
	if !given {
		return errors.New("--users is required and replaces the whole set; --users \"\" clears it")
	}
	if *conn == "" {
		return errors.New("--connection is required")
	}
	ids := []protocol.UserID{}
	for _, u := range strings.Split(*users, ",") {
		if u = strings.TrimSpace(u); u != "" {
			ids = append(ids, u)
		}
	}
	s, err := open(*pod)
	if err != nil {
		return err
	}
	out, err := s.a.SetApprovers(s.token, s.pod, protocol.ApproversRequest{ConnectionID: *conn, UserIds: ids})
	if err != nil {
		return refusal(err)
	}
	if len(out.UserIds) == 0 {
		fmt.Fprintf(stdout, "approvers of %s: none (cleared); only pod owners approve its writes\n", out.ConnectionID)
		return nil
	}
	fmt.Fprintf(stdout, "approvers of %s: %s\n", out.ConnectionID, strings.Join(out.UserIds, ", "))
	return nil
}
